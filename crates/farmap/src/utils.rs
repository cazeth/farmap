pub fn distribution_from_counts<const N: usize>(counts: &[u64; N]) -> Option<[f32; N]> {
    let total: u64 = counts.iter().sum();
    let mut result = [0f32; N];
    if total == 0 {
        return None;
    } else {
        for i in 0..N {
            result[i] = counts[i] as f32 / total as f32;
        }
    }
    Some(result)
}

#[cfg(test)]
pub mod tests {
    use super::*;
    #[test]
    fn distribution_from_counts_test() {
        assert_eq!(distribution_from_counts(&[1, 1]), Some([0.5, 0.5]));
        assert_eq!(
            distribution_from_counts(&[1, 1, 2]),
            Some([0.25, 0.25, 0.5])
        );
        assert_eq!(distribution_from_counts(&[0, 0, 0]), None);
    }
}
