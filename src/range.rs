
#[derive(Debug, PartialEq)]
pub enum RangeOutcome {
    Slice {
        start: u64,
        end: u64
    },
    Unsatisfiable,
}

/// `spec` is the part after "bytes=", e.g. "0-99", "500-", "-100".
pub fn resolve_range(size: u64, spec: &str) -> RangeOutcome {
    use RangeOutcome::*;

    let (start_s, end_s) = match spec.trim().split_once('-') {
        Some(parts) => parts,
        None => return Unsatisfiable, // Empty or no dash
    };

    match (start_s.is_empty(), end_s.is_empty()) {
        (true, true) => Unsatisfiable, // just "-"

        // suffix range: "-N" means the LAST N bytes (not a negative number)
        (true, false) => {
            let n: u64 = match end_s.parse() {
                Ok(n) => n,
                Err(_) => return Unsatisfiable,
            };

            if n == 0 || size == 0 {
                return Unsatisfiable;
            }
            Slice { start: size.saturating_sub(n), end: size - 1 }
        }

        // open-ended: "N-" means from N to the last byte
        (false, true) => {
            let start: u64 = match start_s.parse() {
                Ok(s) => s,
                Err(_) => return Unsatisfiable,
            };

            if start >= size {
                return Unsatisfiable;
            }
            Slice { start, end: size - 1 }
        }

        // closed: "A-B", inclusive on both sides
        (false, false) => {
            let (start, end): (u64, u64) = match (start_s.parse(), end_s.parse()) {
                (Ok(s), Ok(e)) => (s, e),
                _ => return Unsatisfiable,
            };

            if end < start || start >= size {
                return Unsatisfiable;
            }
            Slice { start, end: end.min(size - 1) }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use RangeOutcome::*;

    #[test] fn ordinary_range()        { assert_eq!(resolve_range(1000, "0-99"),     Slice { start: 0,   end: 99 }); }
    #[test] fn end_past_file_clamps()  { assert_eq!(resolve_range(1000, "0-9999"),   Slice { start: 0,   end: 999 }); }
    #[test] fn open_ended()            { assert_eq!(resolve_range(1000, "500-"),     Slice { start: 500, end: 999 }); }
    #[test] fn suffix_range()          { assert_eq!(resolve_range(1000, "-100"),     Slice { start: 900, end: 999 }); }
    #[test] fn suffix_bigger_than_file() { assert_eq!(resolve_range(1000, "-2000"),  Slice { start: 0,   end: 999 }); }
    #[test] fn single_byte()           { assert_eq!(resolve_range(1000, "5-5"),      Slice { start: 5,   end: 5 }); }
    #[test] fn start_beyond_end()      { assert_eq!(resolve_range(1000, "2000-3000"), Unsatisfiable); }
    #[test] fn start_at_size()         { assert_eq!(resolve_range(1000, "1000-"),    Unsatisfiable); }
    #[test] fn reversed_pair()         { assert_eq!(resolve_range(1000, "500-400"),  Unsatisfiable); }
    #[test] fn empty_spec()            { assert_eq!(resolve_range(1000, ""),         Unsatisfiable); }
    #[test] fn garbage_spec()          { assert_eq!(resolve_range(1000, "abc"),      Unsatisfiable); }
    #[test] fn only_dash()             { assert_eq!(resolve_range(1000, "-"),        Unsatisfiable); }
    #[test] fn zero_suffix()           { assert_eq!(resolve_range(1000, "-0"),       Unsatisfiable); }
    #[test] fn empty_file()            { assert_eq!(resolve_range(0, "0-"),          Unsatisfiable); }
}