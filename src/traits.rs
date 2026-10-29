pub trait SplitAtNth {
    fn split_at_nth(&self, delimiter: char, nth: usize) -> Option<(&str, &str)>;
}

impl SplitAtNth for &str {
    fn split_at_nth(&self, delimiter: char, nth: usize) -> Option<(&str, &str)> {
        let mut count = 0;
        for (i, c) in self.char_indices() {
            if c == delimiter {
                count += 1;
                if count == nth {
                    return Some((&self[..i], &self[i + c.len_utf8()..]));
                }
            }
        }
        None
    }
}

#[cfg(test)]
mod test {
    use super::SplitAtNth;

    #[test]
    fn split_at_2nd_occurrence() {
        let item = "123:456:789:012";
        let delimiter = ':';
        let occurrences = 2;

        let output = item.split_at_nth(delimiter, occurrences);
        assert_eq!(output, Some(("123:456", "789:012")))
    }

    #[test]
    fn split_at_10th_occurrence() {
        let item = "1:2:3:4:5:6:7:8:9:0:1:2";
        let delimiter = ':';
        let occurrences = 10;

        let output = item.split_at_nth(delimiter, occurrences);
        assert_eq!(output, Some(("1:2:3:4:5:6:7:8:9:0", "1:2")))
    }

    #[test]
    fn split_utf8() {
        let item = "1😎2😎3😎4";
        let delimiter = '😎';
        let occurrences = 2;

        let output = item.split_at_nth(delimiter, occurrences);
        assert_eq!(output, Some(("1😎2", "3😎4")))
    }

    #[test]
    fn split_zero() {
        let item = "1:2:3:4";
        let delimiter = ':';
        let occurrences = 0;

        let output = item.split_at_nth(delimiter, occurrences);
        assert_eq!(output, None)
    }

    #[test]
    fn split_less_than_occurrence() {
        let item = "1:2:3:4";
        let delimiter = ':';
        let occurrences = 5;

        let output = item.split_at_nth(delimiter, occurrences);
        assert_eq!(output, None)
    }
}
