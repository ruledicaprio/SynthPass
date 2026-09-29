//! Pixel evidence for covered MRZ name cells (ADR-0026).
//!
//! This module measures pixels only. The caller supplies the bounding box of
//! an already verified MRZ line and its row pitch; the name line is located by
//! row offset, without fitting its possibly covered text. Later pipeline and
//! benchmark work chooses thresholds and applies a mask to parsed data.

use image::GrayImage;

use crate::BBox;

/// Pixel measurements in one cell of the name line. Variance and edge detail
/// are divided by the median of the verified line's cells. A uniform fill of
/// any tone has zero variance and edge detail; blur suppresses edge detail.
/// These are signals, not a decision that a cell is covered.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CellPixelFeatures {
    pub mean_luma: f32,
    pub variance: f32,
    pub edge_detail: f32,
    pub relative_variance: f32,
    pub relative_edge_detail: f32,
}

#[derive(Debug, Clone, Copy)]
struct RawFeatures {
    mean: f32,
    variance: f32,
    edge: f32,
}

/// Measure a name line using the verified line's fixed-width cell grid.
///
/// `row_offset` is -1 for the line above a verified line 2, or +2 for TD1's
/// line 3 relative to a verified line 1. `row_pitch` is measured from the
/// zone's line geometry in image pixels, not from the name text. Returns
/// `None` for invalid/out-of-image geometry, cells too small to measure, or
/// a verified line with no measurable texture. This function does no OCR and
/// does not infer a row from damaged text.
pub fn name_cell_features(
    image: &GrayImage,
    verified_line: BBox,
    cell_count: usize,
    row_offset: i32,
    row_pitch: f32,
) -> Option<Vec<CellPixelFeatures>> {
    if cell_count == 0
        || !verified_line.x.is_finite()
        || !verified_line.y.is_finite()
        || !verified_line.w.is_finite()
        || !verified_line.h.is_finite()
        || !row_pitch.is_finite()
        || verified_line.w <= 0.0
        || verified_line.h <= 0.0
        || row_pitch <= 0.0
    {
        return None;
    }

    let name_y = verified_line.y + row_offset as f32 * row_pitch;
    let verified = measure_row(image, verified_line, cell_count)?;
    let name = measure_row(
        image,
        BBox {
            y: name_y,
            ..verified_line
        },
        cell_count,
    )?;
    let mut variances: Vec<f32> = verified.iter().map(|cell| cell.variance).collect();
    let mut edges: Vec<f32> = verified.iter().map(|cell| cell.edge).collect();
    variances.sort_by(f32::total_cmp);
    edges.sort_by(f32::total_cmp);
    let reference_variance = variances[variances.len() / 2];
    let reference_edge = edges[edges.len() / 2];
    if reference_variance <= f32::EPSILON || reference_edge <= f32::EPSILON {
        return None;
    }

    Some(
        name.into_iter()
            .map(|cell| CellPixelFeatures {
                mean_luma: cell.mean,
                variance: cell.variance,
                edge_detail: cell.edge,
                relative_variance: cell.variance / reference_variance,
                relative_edge_detail: cell.edge / reference_edge,
            })
            .collect(),
    )
}

fn measure_row(image: &GrayImage, line: BBox, count: usize) -> Option<Vec<RawFeatures>> {
    let y0 = line.y.ceil() as i64;
    let y1 = (line.y + line.h).floor() as i64;
    if y0 < 0 || y1 > i64::from(image.height()) || y1 - y0 < 3 {
        return None;
    }
    let mut cells = Vec::with_capacity(count);
    for index in 0..count {
        let left = line.x + line.w * index as f32 / count as f32;
        let right = line.x + line.w * (index + 1) as f32 / count as f32;
        let x0 = left.ceil() as i64;
        let x1 = right.floor() as i64;
        if x0 < 0 || x1 > i64::from(image.width()) || x1 - x0 < 3 {
            return None;
        }
        let (x0, x1, y0, y1) = (x0 as u32, x1 as u32, y0 as u32, y1 as u32);
        let n = (x1 - x0) as f32 * (y1 - y0) as f32;
        let mut sum = 0.0f32;
        let mut sum_sq = 0.0f32;
        let mut detail = 0.0f32;
        let mut detail_count = 0u32;
        for y in y0..y1 {
            for x in x0..x1 {
                let p = f32::from(image.get_pixel(x, y)[0]);
                sum += p;
                sum_sq += p * p;
                if x > x0 && x + 1 < x1 && y > y0 && y + 1 < y1 {
                    let neighbors = f32::from(image.get_pixel(x - 1, y)[0])
                        + f32::from(image.get_pixel(x + 1, y)[0])
                        + f32::from(image.get_pixel(x, y - 1)[0])
                        + f32::from(image.get_pixel(x, y + 1)[0]);
                    detail += (p - neighbors / 4.0).abs();
                    detail_count += 1;
                }
            }
        }
        let mean = sum / n;
        cells.push(RawFeatures {
            mean,
            variance: (sum_sq / n - mean * mean).max(0.0),
            edge: detail / detail_count as f32,
        });
    }
    Some(cells)
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::Luma;

    const CELL: u32 = 16;

    fn printed_rows() -> GrayImage {
        let mut image = GrayImage::from_pixel(4 * CELL, 2 * CELL, Luma([240]));
        for row in 0..2 {
            for cell in 0..4 {
                let x0 = cell * CELL;
                let y0 = row * CELL;
                for y in 2..CELL - 2 {
                    for x in 2..CELL - 2 {
                        if x == 4 || x == 9 || y == 8 {
                            image.put_pixel(x0 + x, y0 + y, Luma([20]));
                        }
                    }
                }
            }
        }
        image
    }

    fn features(image: &GrayImage) -> Vec<CellPixelFeatures> {
        name_cell_features(
            image,
            BBox {
                x: 0.0,
                y: CELL as f32,
                w: (4 * CELL) as f32,
                h: CELL as f32,
            },
            4,
            -1,
            CELL as f32,
        )
        .expect("constructed verified line is textured")
    }

    #[test]
    fn fill_signal_is_independent_of_tone() {
        let mut image = printed_rows();
        for (cell, tone) in [0, 128, 255].into_iter().enumerate() {
            for y in 0..CELL {
                for x in (cell as u32 * CELL)..((cell as u32 + 1) * CELL) {
                    image.put_pixel(x, y, Luma([tone]));
                }
            }
        }
        let measured = features(&image);
        for (cell, tone) in [0, 128, 255].into_iter().enumerate() {
            assert_eq!(measured[cell].mean_luma, tone as f32);
            assert_eq!(measured[cell].relative_variance, 0.0);
            assert_eq!(measured[cell].relative_edge_detail, 0.0);
        }
        assert!(measured[3].relative_variance > 0.5);
        assert!(measured[3].relative_edge_detail > 0.5);
    }

    #[test]
    fn blur_loses_detail_against_the_verified_line() {
        let original = printed_rows();
        let mut blurred = original.clone();
        for y in 1..CELL - 1 {
            for x in 1..CELL - 1 {
                let mut sum = 0u32;
                for yy in y - 1..=y + 1 {
                    for xx in x - 1..=x + 1 {
                        sum += u32::from(original.get_pixel(xx, yy)[0]);
                    }
                }
                blurred.put_pixel(x, y, Luma([(sum / 9) as u8]));
            }
        }
        let ordinary = features(&original);
        let measured = features(&blurred);
        assert!(measured[0].relative_edge_detail < ordinary[0].relative_edge_detail);
        assert!(measured[0].relative_variance < ordinary[0].relative_variance);
        assert_eq!(measured[1], ordinary[1]);
    }

    #[test]
    fn invalid_geometry_and_untextured_reference_do_not_claim_coverage() {
        let blank = GrayImage::from_pixel(4 * CELL, 2 * CELL, Luma([255]));
        let verified = BBox {
            x: 0.0,
            y: CELL as f32,
            w: (4 * CELL) as f32,
            h: CELL as f32,
        };
        assert!(name_cell_features(&blank, verified, 4, -1, CELL as f32).is_none());
        let printed = printed_rows();
        assert!(name_cell_features(&printed, verified, 0, -1, CELL as f32).is_none());
        assert!(name_cell_features(&printed, verified, 4, -2, CELL as f32).is_none());
        assert!(name_cell_features(&printed, verified, 4, -1, f32::NAN).is_none());
    }
}
