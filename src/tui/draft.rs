//! Multiline message draft with a cursor, independent of any terminal.
use unicode_width::UnicodeWidthChar;

#[derive(Default)]
pub struct Draft {
    text: String,
    /// Byte offset, always on a char boundary.
    cursor: usize,
}

impl Draft {
    pub fn text(&self) -> &str {
        &self.text
    }
    pub fn is_blank(&self) -> bool {
        self.text.trim().is_empty()
    }
    /// Returns the text and leaves an empty draft.
    pub fn take(&mut self) -> String {
        self.cursor = 0;
        std::mem::take(&mut self.text)
    }
    pub fn insert(&mut self, c: char) {
        let c = if c == '\t' { ' ' } else { c };
        if c == '\n' || !c.is_control() {
            self.text.insert(self.cursor, c);
            self.cursor += c.len_utf8();
        }
    }
    /// Pasted text keeps its line breaks; other control characters are dropped.
    pub fn paste(&mut self, text: &str) {
        for c in text.replace("\r\n", "\n").replace('\r', "\n").chars() {
            self.insert(c);
        }
    }
    pub fn backspace(&mut self) {
        if let Some(c) = self.text[..self.cursor].chars().next_back() {
            self.cursor -= c.len_utf8();
            self.text.remove(self.cursor);
        }
    }
    pub fn delete(&mut self) {
        if self.cursor < self.text.len() {
            self.text.remove(self.cursor);
        }
    }
    pub fn left(&mut self) {
        if let Some(c) = self.text[..self.cursor].chars().next_back() {
            self.cursor -= c.len_utf8();
        }
    }
    pub fn right(&mut self) {
        if let Some(c) = self.text[self.cursor..].chars().next() {
            self.cursor += c.len_utf8();
        }
    }
    fn line_start(&self) -> usize {
        self.text[..self.cursor].rfind('\n').map_or(0, |i| i + 1)
    }
    fn line_end(&self) -> usize {
        self.text[self.cursor..]
            .find('\n')
            .map_or(self.text.len(), |i| self.cursor + i)
    }
    pub fn home(&mut self) {
        self.cursor = self.line_start();
    }
    pub fn end(&mut self) {
        self.cursor = self.line_end();
    }
    /// Byte offset in the line starting at `start` whose display column is
    /// closest to `column` without passing it (wide chars take two cells).
    fn at_column(&self, start: usize, column: usize) -> usize {
        let line = &self.text[start..];
        let line = &line[..line.find('\n').unwrap_or(line.len())];
        let mut used = 0;
        for (i, c) in line.char_indices() {
            let w = c.width().unwrap_or(0);
            if used + w > column {
                return start + i;
            }
            used += w;
        }
        start + line.len()
    }
    /// Display column of the cursor, in terminal cells.
    fn column(&self) -> usize {
        self.text[self.line_start()..self.cursor]
            .chars()
            .map(|c| c.width().unwrap_or(0))
            .sum()
    }
    /// Returns false on the first line, so the caller can use Up elsewhere.
    pub fn up(&mut self) -> bool {
        let start = self.line_start();
        if start == 0 {
            return false;
        }
        let column = self.column();
        let previous = self.text[..start - 1].rfind('\n').map_or(0, |i| i + 1);
        self.cursor = self.at_column(previous, column);
        true
    }
    /// Returns false on the last line.
    pub fn down(&mut self) -> bool {
        let end = self.line_end();
        if end == self.text.len() {
            return false;
        }
        let column = self.column();
        self.cursor = self.at_column(end + 1, column);
        true
    }
    /// Rows wrapped to `width` display cells, and the cursor as (row, col).
    pub fn layout(&self, width: u16) -> (Vec<String>, (usize, u16)) {
        let width = width.max(1) as usize;
        let mut rows = vec![String::new()];
        let mut used = 0;
        let mut cursor = (0, 0);
        for (i, c) in self.text.char_indices() {
            if i == self.cursor {
                cursor = (rows.len() - 1, used as u16);
            }
            if c == '\n' {
                rows.push(String::new());
                used = 0;
                continue;
            }
            let w = c.width().unwrap_or(0);
            if used + w > width {
                rows.push(String::new());
                used = 0;
                if i == self.cursor {
                    cursor = (rows.len() - 1, 0);
                }
            }
            rows.last_mut().expect("one row").push(c);
            used += w;
        }
        if self.cursor == self.text.len() {
            if used >= width {
                rows.push(String::new());
                used = 0;
            }
            cursor = (rows.len() - 1, used as u16);
        }
        (rows, cursor)
    }
}

/// Splits a line into rows of at most `width` display cells.
pub fn wrap(line: &str, width: u16) -> Vec<String> {
    let width = width.max(1) as usize;
    let mut rows = vec![String::new()];
    let mut used = 0;
    for c in line.chars() {
        let w = c.width().unwrap_or(0);
        if used + w > width {
            rows.push(String::new());
            used = 0;
        }
        rows.last_mut().expect("one row").push(c);
        used += w;
    }
    rows
}

#[cfg(test)]
mod tests {
    use super::*;

    fn draft(text: &str) -> Draft {
        let mut d = Draft::default();
        d.paste(text);
        d
    }

    #[test]
    fn edits_across_lines_keep_the_column() {
        let mut d = draft("primeira\nb\nterceira");
        assert!(d.up());
        assert_eq!(d.layout(80).1, (1, 1));
        assert!(d.up());
        assert_eq!(d.layout(80).1, (0, 1), "column follows the short line");
        assert!(!d.up());
        d.home();
        d.insert('>');
        d.end();
        d.backspace();
        assert!(d.down() && d.down());
        assert!(!d.down());
        d.left();
        d.delete();
        assert_eq!(d.text(), ">primeir\nb\nerceira");
    }

    #[test]
    fn vertical_moves_keep_the_visual_column_with_wide_chars() {
        let mut d = draft("漢字漢\nabcdef");
        d.left();
        d.left();
        assert_eq!(d.layout(80).1, (1, 4));
        assert!(d.up());
        assert_eq!(d.layout(80).1, (0, 4), "lands after two wide chars");
        assert!(d.down());
        assert_eq!(d.layout(80).1, (1, 4));
        d.left();
        assert!(d.up());
        assert_eq!(d.layout(80).1, (0, 2), "never inside a wide char");
    }

    #[test]
    fn paste_normalizes_breaks_and_drops_controls() {
        let mut d = draft("a\r\nb\rc\x1b[31m\td");
        assert_eq!(d.text(), "a\nb\nc[31m d");
        assert!(!d.is_blank());
        assert_eq!(d.take(), "a\nb\nc[31m d");
        assert!(d.is_blank() && d.text().is_empty());
        d.paste(" \n ");
        assert!(d.is_blank());
    }

    #[test]
    fn layout_wraps_wide_chars_and_places_the_cursor() {
        let mut d = draft("abcdé漢字");
        let (rows, cursor) = d.layout(4);
        assert_eq!(rows, ["abcd", "é漢", "字"]);
        assert_eq!(cursor, (2, 2));
        d.home();
        d.right();
        d.right();
        d.right();
        d.right();
        assert_eq!(d.layout(4).1, (1, 0));
        assert_eq!(
            draft("abcd").layout(4),
            (vec!["abcd".into(), String::new()], (1, 0))
        );
        assert_eq!(wrap("漢字漢", 3), ["漢", "字", "漢"]);
        assert_eq!(wrap("", 3), [""]);
    }
}
