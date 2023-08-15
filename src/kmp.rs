pub const fn search(haystack: &[u8], needle: &[u8], table: &[usize]) -> Option<usize> {
    let mut t_i = 0;
    let mut p_i = 0;
    let mut result_idx = 0;

    while t_i < haystack.len() && p_i < needle.len() {
        if haystack[t_i] == needle[p_i] {
            if result_idx == 0 {
                result_idx = t_i;
            }
            t_i = t_i + 1;
            p_i = p_i + 1;
            if p_i >= needle.len() {
                return Some(result_idx);
            }
        } else {
            if p_i == 0 {
                p_i = 0;
            } else {
                p_i = table[p_i - 1];
            }
            t_i = t_i + 1;
            result_idx = 0;
        }
    }
    None
}

pub const fn table<const N: usize>(p: &[u8]) -> [usize; N] {
    let m = p.len();
    let mut t = [0; N];

    let mut i = 1;
    let mut j = 0;
    while i < m {
        if p[i] == p[j] {
            t[i] = j + 1;
            i = i + 1;
            j = j + 1;
        } else if j == 0 {
            t[i] = 0;
            i = i + 1;
        } else {
            j = t[j - 1];
        }
    }
    t
}
