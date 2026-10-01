use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

/// Ellipsis appended to cut text.
const ELLIPSIS: char = '…';

/// Terminal columns the ellipsis occupies.
const ELLIPSIS_WIDTH: usize = 1;

/// Longest prefix of `text` that fits in `max_width` terminal columns.
///
/// Cut text ends in `…` within the limit. A character is never split; a wide
/// character that does not fit is dropped. Control characters become spaces.
pub fn truncate_to_width(text: &str, max_width: usize) -> String {
    let cleaned: String = text
        .chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect();
    if cleaned.width() <= max_width {
        return cleaned;
    }
    if max_width == 0 {
        return String::new();
    }
    let budget = max_width - ELLIPSIS_WIDTH;
    let mut cut = String::new();
    let mut used = 0;
    for c in cleaned.chars() {
        let width = c.width().unwrap_or(0);
        if used + width > budget {
            break;
        }
        cut.push(c);
        used += width;
    }
    cut.push(ELLIPSIS);
    cut
}

/// Draw `text` on row `y`, centered on column `center_x` and kept inside `area`.
///
/// Text wider than `area` is cut to its width. Rows outside `area` draw nothing.
pub fn draw_centered(
    buf: &mut Buffer,
    area: Rect,
    center_x: i32,
    y: i32,
    text: &str,
    style: Style,
) {
    let row_inside = y >= i32::from(area.top()) && y < i32::from(area.bottom());
    if !row_inside {
        return;
    }
    let text = truncate_to_width(text, usize::from(area.width));
    let width = i32::try_from(text.width()).unwrap_or(i32::MAX);
    let last_start = i32::from(area.right()) - width;
    let start = (center_x - width / 2).clamp(i32::from(area.left()), last_start);
    // Both bounds were checked against `area`, so the casts cannot truncate.
    buf.set_stringn(
        start as u16,
        y as u16,
        &text,
        usize::from(area.width),
        style,
    );
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case("", 8, "")] // empty
    #[case("farm", 8, "farm")] // fits
    #[case("12345678", 8, "12345678")] // exactly the limit
    #[case("123456789", 8, "1234567…")] // one over
    #[case("你好世界", 8, "你好世界")] // wide text that exactly fits
    #[case("你好世界你好", 8, "你好世…")] // cut leaves one column for the ellipsis
    #[case("你好世界", 4, "你…")] // odd budget drops the wide character that does not fit
    #[case("abc", 0, "")] // no room at all
    #[case("a\tb", 8, "a b")] // control characters
    fn truncates_by_display_width(#[case] text: &str, #[case] max: usize, #[case] want: &str) {
        let got = truncate_to_width(text, max);
        assert_eq!(got, want);
        assert!(got.width() <= max, "{got:?} wider than {max}");
    }

    fn row(buf: &Buffer, y: u16) -> String {
        (0..buf.area.width)
            .map(|x| buf[(x, y)].symbol().to_owned())
            .collect()
    }

    #[test]
    fn centered_wide_text_is_written_whole() {
        let area = Rect::new(0, 0, 20, 3);
        let mut buf = Buffer::empty(area);
        draw_centered(&mut buf, area, 10, 1, "你好", Style::default());
        assert_eq!(buf[(8, 1)].symbol(), "你");
        assert_eq!(buf[(10, 1)].symbol(), "好");
    }

    #[rstest]
    #[case(-5, 0)] // centre left of the area
    #[case(100, 16)] // centre right of the area
    fn text_is_shifted_to_stay_inside_the_area(#[case] center: i32, #[case] first_col: u16) {
        let area = Rect::new(0, 0, 20, 3);
        let mut buf = Buffer::empty(area);
        draw_centered(&mut buf, area, center, 0, "abcd", Style::default());
        assert_eq!(row(&buf, 0).trim(), "abcd");
        assert_eq!(buf[(first_col, 0)].symbol(), "a");
    }

    #[rstest]
    #[case(-1)]
    #[case(3)]
    fn rows_outside_the_area_draw_nothing(#[case] y: i32) {
        let area = Rect::new(0, 0, 20, 3);
        let mut buf = Buffer::empty(area);
        draw_centered(&mut buf, area, 10, y, "abcd", Style::default());
        assert_eq!(buf, Buffer::empty(area));
    }
}
