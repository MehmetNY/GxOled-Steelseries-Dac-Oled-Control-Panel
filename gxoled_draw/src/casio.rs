//! Monochrome 128x64 Casio-inspired clock, based on the provided pixel reference.
//! Intentionally no grayscale or dithering: device pixels are either on or off.
use chrono::{DateTime, Datelike, Local, Timelike};
use gxoled_lib::Bitmap;

fn dot(dst: &mut Bitmap, x: i32, y: i32) {
    if x >= 0 && y >= 0 && (x as usize) < dst.w && (y as usize) < dst.h {
        dst.data.set(y as usize * dst.w + x as usize, true);
    }
}
fn rect(dst: &mut Bitmap, x: i32, y: i32, w: i32, h: i32) {
    for py in y..y+h { for px in x..x+w { dot(dst, px, py); } }
}

// A thin 3x5 pixel font for date, day and the lower seconds counter.
fn font(ch: char) -> [u8; 5] {
    match ch {
        'A' => [2,5,7,5,5], 'B' => [6,5,6,5,6], 'C' => [3,4,4,4,3],
        'E' => [7,4,6,4,7], 'F' => [7,4,6,4,4], 'I' => [7,2,2,2,7],
        'M' => [5,7,7,5,5], 'N' => [5,7,7,7,5], 'P' => [6,5,6,4,4],
        'R' => [6,5,6,5,5], 'S' => [3,4,2,1,6], 'T' => [7,2,2,2,2],
        'U' => [5,5,5,5,7], 'Z' => [7,1,2,4,7], ' ' => [0;5],
        '0' => [7,5,5,5,7], '1' => [2,6,2,2,7], '2' => [7,1,7,4,7],
        '3' => [7,1,7,1,7], '4' => [5,5,7,1,1], '5' => [7,4,7,1,7],
        '6' => [7,4,7,5,7], '7' => [7,1,1,1,1], '8' => [7,5,7,5,7],
        '9' => [7,5,7,1,7], _ => [0;5],
    }
}
fn text(dst: &mut Bitmap, content: &str, x: i32, y: i32, scale: i32) {
    for (idx, ch) in content.chars().enumerate() {
        for (row, bits) in font(ch).iter().enumerate() {
            for col in 0..3i32 {
                if (*bits & (1u8 << (2 - col))) != 0 {
                    rect(dst, x + idx as i32 * 4 * scale + col*scale,
                        y + row as i32*scale, scale, scale);
                }
            }
        }
    }
}

fn font5(ch: char) -> [u8; 7] {
    match ch {
        'B' => [30,17,17,30,17,17,30],
        'D' => [30,17,17,17,17,17,30],
        'F' => [31,16,16,30,16,16,16],
        'G' => [14,17,16,23,17,17,14],
        'H' => [17,17,17,31,17,17,17],
        'I' => [31,4,4,4,4,4,31],
        'J' => [7,2,2,2,18,18,12],
        'K' => [17,18,20,24,20,18,17],
        'L' => [16,16,16,16,16,16,31],
        'N' => [17,25,21,19,17,17,17],
        'O' => [14,17,17,17,17,17,14],
        'Q' => [14,17,17,17,21,18,13],
        'V' => [17,17,17,17,17,10,4],
        'W' => [17,17,17,21,21,21,10],
        'X' => [17,17,10,4,10,17,17],
        'Y' => [17,17,10,4,4,4,4],
        'P' => [30, 17, 17, 30, 16, 16, 16],
        'E' => [31, 16, 16, 30, 16, 16, 31],
        'R' => [30, 17, 17, 30, 20, 18, 17],
        '0' => [14, 17, 19, 21, 25, 17, 14],
        'C' => [14, 17, 16, 16, 16, 17, 14],
        'A' => [14, 17, 17, 31, 17, 17, 17],
        'M' => [17, 27, 21, 17, 17, 17, 17],
        'Z' => [31, 1, 2, 4, 8, 16, 31],
        'T' => [31, 4, 4, 4, 4, 4, 4],
        'S' => [14, 17, 16, 14, 1, 17, 14],
        'U' => [17, 17, 17, 17, 17, 17, 14],
        '1' => [4, 12, 4, 4, 4, 4, 14],
        '2' => [14, 17, 1, 2, 4, 8, 31],
        '3' => [14, 17, 1, 6, 1, 17, 14],
        '4' => [2, 6, 10, 18, 31, 2, 2],
        '5' => [31, 16, 30, 1, 1, 17, 14],
        '6' => [6, 8, 16, 30, 17, 17, 14],
        '7' => [31, 1, 2, 4, 8, 8, 8],
        '8' => [14, 17, 17, 14, 17, 17, 14],
        '9' => [14, 17, 17, 15, 1, 2, 12],
        ' ' => [0;7], _ => [0;7],
    }
}
fn header_text(dst: &mut Bitmap, content: &str) {
    for (i,ch) in content.chars().enumerate() {
        for (row, bits) in font5(ch).iter().enumerate() {
            for col in 0..5i32 {
                if (*bits & (1u8 << (4-col))) != 0 {
                    dot(dst, 2 + i as i32*6 + col, 2+row as i32);
                }
            }
        }
    }
}

// Standard 7-segment LED geometry, exactly three OLED pixels thick.
const SEGMENTS: [u8; 10] = [
    0b1111110, 0b0110000, 0b1101101, 0b1111001, 0b0110011,
    0b1011011, 0b1011111, 0b1110000, 0b1111111, 0b1111011,
];
fn big_digit(dst: &mut Bitmap, x: i32, y: i32, digit: u32) {
    // The original 7-segment digit 1 looks like two detached vertical strokes.
    // Give it a continuous LCD stem, a short angled head and a clear footing.
    if digit == 1 {
        rect(dst, x + 8, y + 5, 3, 3);
        rect(dst, x + 11, y + 2, 3, 24);
        rect(dst, x + 6, y + 25, 14, 3);
        return;
    }
    let seg = SEGMENTS[digit as usize];
    let h = [(x+3,y,18,3), (x+21,y+3,3,10), (x+21,y+15,3,10),
             (x+3,y+25,18,3), (x,y+15,3,10), (x,y+3,3,10), (x+3,y+12,18,3)];
    for (idx, (px,py,w,h)) in h.iter().enumerate() {
        if seg & (1 << (6-idx)) != 0 { rect(dst,*px,*py,*w,*h); }
    }
}

pub fn render(now: DateTime<Local>, twelve_hour: bool, turkish: bool) -> Bitmap {
    let mut out = Bitmap::new(128, 64, false);
    // Same horizontal separators as the reference image.
    rect(&mut out, 2, 10, 124, 1);
    rect(&mut out, 2, 44, 124, 1);
    let days = if turkish { ["PAZ", "PZT", "SAL", "CAR", "PER", "CUM", "CMT"] }
        else { ["SUN", "MON", "TUE", "WED", "THU", "FRI", "SAT"] };
    let day = days[now.weekday().num_days_from_sunday() as usize];
    header_text(&mut out, &format!("{day} {:02}", now.day()));
    // Four week-of-month indicators. Days 29-31 are included in the fourth band.
    let week = ((now.day()-1) / 7 + 1).min(4);
    for i in 0..4 { if (i+1) <= week { rect(&mut out, 95+i as i32*8, 3, 3, 3); } }
    let hour = if twelve_hour { ((now.hour()+11)%12)+1 } else { now.hour() };
    // Keep at least two clear pixel rows between large digits and both rules.
    big_digit(&mut out, 7, 14, hour / 10);
    big_digit(&mut out, 35, 14, hour % 10);
    if now.second() % 2 == 0 {
        rect(&mut out, 63, 21, 2, 3);
        rect(&mut out, 63, 32, 2, 3);
    }
    big_digit(&mut out, 71, 14, now.minute() / 10);
    big_digit(&mut out, 99, 14, now.minute() % 10);
    text(&mut out, &format!("{:02}", now.second()), 6, 50, 2);
    // Exactly 60 individual 1-pixel ticks advancing from left to right.
    // Each pair is separated from the next pair to make them distinguishable.
    for i in 0..=now.second() {
        let px = 32 + i as i32 + i as i32 / 2;
        rect(&mut out, px, 53, 1, 6);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    #[test]
    fn fills_sixty_ticks_and_uses_stable_geometry() {
        let t = Local.with_ymd_and_hms(2026, 10, 22, 12, 45, 59).single().unwrap();
        let frame = render(t, false, false);
        assert_eq!((frame.w, frame.h), (128, 64));
        assert!(frame.data[53*128+32]);
        assert!(frame.data[53*128+32+59+59/2]);
        let next = render(t + chrono::Duration::seconds(1), false, false);
        assert!(next.data[53*128+32]);
        assert!(!next.data[53*128+32+59+59/2]);
    }
}
