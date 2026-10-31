//! The tray icon, rasterised from the Shadow logo geometry (an eclipse crescent inside a
//! faint corona), so no image files or decoders are needed.
//!
//! * protecting: solid crescent + corona
//! * not running: hollow crescent outline, no corona
//!
//! macOS uses it as a template image (black + alpha, tinted by the system); Windows gets
//! the brand colour, grey when Shadow is down.

use tray_icon::Icon;

const SIZE: u32 = 64;
const SS: u32 = 4; // supersampling per axis

fn coverage(on: bool, x: f64, y: f64) -> f64 {
    let d = |cx: f64, cy: f64| ((x - cx).powi(2) + (y - cy).powi(2)).sqrt();
    let disc = d(32.0, 32.0);
    let bite = d(41.0, 25.0);
    let in_crescent = disc <= 24.0 && bite > 20.0;
    if on {
        let corona = (disc - 29.0).abs() <= 1.9;
        (in_crescent || corona) as u8 as f64
    } else {
        // 3.5-unit outline of the crescent.
        let edge = in_crescent && (disc >= 20.5 || bite <= 23.5);
        edge as u8 as f64
    }
}

/// RGBA pixels, width, height.
pub fn rgba(on: bool, template: bool) -> (Vec<u8>, u32, u32) {
    let (r, g, b) = if template {
        (0, 0, 0)
    } else if on {
        (0x7c, 0x6c, 0xff)
    } else {
        (0x8a, 0x8a, 0xa3)
    };
    let mut out = Vec::with_capacity((SIZE * SIZE * 4) as usize);
    for py in 0..SIZE {
        for px in 0..SIZE {
            let mut a = 0.0;
            for sy in 0..SS {
                for sx in 0..SS {
                    let x = px as f64 + (sx as f64 + 0.5) / SS as f64;
                    let y = py as f64 + (sy as f64 + 0.5) / SS as f64;
                    a += coverage(on, x, y);
                }
            }
            let alpha = (a / (SS * SS) as f64 * 255.0).round() as u8;
            out.extend_from_slice(&[r, g, b, alpha]);
        }
    }
    (out, SIZE, SIZE)
}

pub fn tray_icon(on: bool) -> Icon {
    let (px, w, h) = rgba(on, cfg!(target_os = "macos"));
    Icon::from_rgba(px, w, h).expect("valid icon")
}
