//! ADR-0030 Decision 5, as amended in #708: deterministic per-field admission.
use super::{load_fonts, Fonts, VizFont};
use crate::data::{viz_domain, VizDomain};
use crate::layout::{widest_ink_at_scale, LayoutField, Rect, ValidatedLayout};
use crate::model::DocumentType;
use crate::render::viz_px_scale;

const BISECTION_STEPS: usize = 24;
const STEP_DOWN: f32 = 0.00001;
const MAX_STEP_DOWNS: usize = 100_000;

/// A field's factor and exact f32 pixel size, computed together once.
#[derive(Debug, Clone, Copy)]
pub struct FieldScale {
    pub k: f32,
    pub px: f32,
    pub binding: &'static str,
}

/// The eleven canonical layout slots (portrait's unused size is zero).
#[derive(Debug, Clone)]
pub struct FieldScales(pub(crate) [FieldScale; 11]);

impl FieldScales {
    pub fn field(&self, field: LayoutField) -> FieldScale {
        self.0[field as usize]
    }
}

/// Largest right/bottom-admissible factor, stepped down for rounded top/left.
/// The stored product is also the renderer's product.
pub(crate) fn compute_field_scale(
    fonts: &Fonts,
    family: VizFont,
    rect: Rect,
    domain: &VizDomain,
) -> Option<FieldScale> {
    let px = viz_px_scale(rect);
    if family == VizFont::PtSans {
        return Some(FieldScale {
            k: 1.0,
            px,
            binding: "cap",
        });
    }
    let reference = widest_ink_at_scale(&fonts.viz, domain, rect, px)?;
    let font = fonts.viz_font(family).ok()?;
    let ink_at = |k| widest_ink_at_scale(font, domain, rect, k * px);
    let right_bottom_ok = |k| {
        ink_at(k).is_some_and(|ink| ink.max_x <= reference.max_x && ink.max_y <= reference.max_y)
    };
    let mut low = 0.0f32;
    let mut high = 1.0f32;
    if right_bottom_ok(high) {
        low = high;
    } else {
        for _ in 0..BISECTION_STEPS {
            let mid = (low + high) / 2.0;
            if right_bottom_ok(mid) {
                low = mid;
            } else {
                high = mid;
            }
        }
    }
    let binding = if low == 1.0 {
        "cap"
    } else {
        let beyond = ink_at((low + 0.001).min(1.0))?;
        match (
            beyond.max_x > reference.max_x,
            beyond.max_y > reference.max_y,
        ) {
            (true, true) => "right+bottom",
            (true, false) => "right",
            (false, true) => "bottom",
            (false, false) => "rounding",
        }
    };
    for step in 0..MAX_STEP_DOWNS {
        let k = low - step as f32 * STEP_DOWN;
        if k <= 0.0 {
            return None;
        }
        let ink = ink_at(k)?;
        if ink.min_x >= rect.x as i32
            && ink.min_y >= rect.y as i32
            && ink.max_x <= reference.max_x
            && ink.max_y <= reference.max_y
        {
            return Some(FieldScale {
                k,
                px: k * px,
                binding,
            });
        }
    }
    None
}

pub(crate) fn compute_layout_scales(
    layout: &ValidatedLayout,
    family: VizFont,
) -> Result<FieldScales, String> {
    compute_layout_with_domains(layout, family, viz_domain)
}

pub(crate) fn compute_layout_with_domains(
    layout: &ValidatedLayout,
    family: VizFont,
    domain_for: impl Fn(DocumentType, LayoutField) -> Option<VizDomain>,
) -> Result<FieldScales, String> {
    let mut scales = FieldScales(
        [FieldScale {
            k: 1.0,
            px: 0.0,
            binding: "cap",
        }; 11],
    );
    if family == VizFont::PtSans {
        for field in LayoutField::ALL {
            if field.is_visual_zone() {
                scales.0[field as usize].px = viz_px_scale(layout.spec().rect(field));
            }
        }
        return Ok(scales);
    }
    let fonts = load_fonts().map_err(|error| error.to_string())?;
    for field in LayoutField::ALL {
        let Some(domain) = domain_for(layout.format(), field) else {
            continue;
        };
        scales.0[field as usize] =
            compute_field_scale(&fonts, family, layout.spec().rect(field), &domain).ok_or_else(
                || {
                    format!(
                        "VIZ font {} cannot draw {} field {} at any admissible scale",
                        family.name(),
                        layout.format().as_str(),
                        field.name()
                    )
                },
            )?;
    }
    Ok(scales)
}
