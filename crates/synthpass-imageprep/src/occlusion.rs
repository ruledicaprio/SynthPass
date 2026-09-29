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
/// `row_offset` is -1 for the name line above a verified line 2, or +1 for
/// TD1's line 3 below its verified line 2. `row_pitch` is measured from the
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

/// Measure `count` equal cells across `line`, from `line.x` to
/// `line.x + line.w`.
///
/// The grid assumes that `line` spans exactly `count` cell pitches. An OCR
/// line box is the ink extent instead, from the first glyph's left edge to the
/// last glyph's right edge, which is about one inter-glyph gap short of
/// `count` pitches. On such a box the cells come out slightly narrow and drift
/// against the printed cells by up to about one gap across the line. A caller
/// that passes an OCR box should measure that drift before choosing a
/// threshold.
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
        // Pixels are `u8`, so every sum is an exact integer, converted once at
        // the end. An `f32` sum stops being exact past 2^24, and would give a
        // 48 x 56 cell filled at 253 a variance of about 0.86.
        let n = u64::from(x1 - x0) * u64::from(y1 - y0);
        let mut sum = 0u64;
        let mut sum_sq = 0u64;
        let mut detail = 0u64;
        let mut detail_count = 0u64;
        for y in y0..y1 {
            for x in x0..x1 {
                let p = u64::from(image.get_pixel(x, y)[0]);
                sum += p;
                sum_sq += p * p;
                if x > x0 && x + 1 < x1 && y > y0 && y + 1 < y1 {
                    let neighbors = u64::from(image.get_pixel(x - 1, y)[0])
                        + u64::from(image.get_pixel(x + 1, y)[0])
                        + u64::from(image.get_pixel(x, y - 1)[0])
                        + u64::from(image.get_pixel(x, y + 1)[0]);
                    // |4p - neighbours| is four times |p - their mean|.
                    detail += (4 * p).abs_diff(neighbors);
                    detail_count += 1;
                }
            }
        }
        // n * sum_sq - sum^2 is n^2 times the variance. It is never negative,
        // and it is exactly 0 for a uniform fill.
        let spread = u128::from(n) * u128::from(sum_sq) - u128::from(sum) * u128::from(sum);
        cells.push(RawFeatures {
            mean: (sum as f64 / n as f64) as f32,
            variance: (spread as f64 / (n as f64 * n as f64)) as f32,
            edge: (detail as f64 / (4 * detail_count) as f64) as f32,
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
        printed_rows_of(CELL, CELL)
    }

    /// Two rows of four `width` x `height` cells, each holding the same glyph.
    fn printed_rows_of(width: u32, height: u32) -> GrayImage {
        let mut image = GrayImage::from_pixel(4 * width, 2 * height, Luma([240]));
        for row in 0..2 {
            for cell in 0..4 {
                let x0 = cell * width;
                let y0 = row * height;
                for y in 2..height - 2 {
                    for x in 2..width - 2 {
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
        features_of(image, CELL, CELL)
    }

    /// The name row's cells, measured against the verified second row.
    fn features_of(image: &GrayImage, width: u32, height: u32) -> Vec<CellPixelFeatures> {
        name_cell_features(
            image,
            BBox {
                x: 0.0,
                y: height as f32,
                w: (4 * width) as f32,
                h: height as f32,
            },
            4,
            -1,
            height as f32,
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

    /// A uniform fill measures exactly 0 on a cell the size of a real MRZ
    /// cell, not only on the 16 x 16 cells above. An `f32` accumulation gives
    /// this 48 x 56 cell at tone 253 a variance of about 0.86.
    #[test]
    fn a_uniform_fill_measures_zero_on_a_large_cell() {
        let (width, height) = (48, 56);
        let mut image = printed_rows_of(width, height);
        for y in 0..height {
            for x in 0..width {
                image.put_pixel(x, y, Luma([253]));
            }
        }
        let measured = features_of(&image, width, height);
        assert_eq!(measured[0].mean_luma, 253.0);
        assert_eq!(measured[0].variance, 0.0);
        assert_eq!(measured[0].relative_variance, 0.0);
        assert_eq!(measured[0].edge_detail, 0.0);
        assert_eq!(measured[0].relative_edge_detail, 0.0);
        assert!(measured[1].relative_variance > 0.5);
    }
}
