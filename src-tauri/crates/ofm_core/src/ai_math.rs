/// Mean of the attributes used by AI squad and role readings.
pub(crate) fn mean_u8(values: impl IntoIterator<Item = u8>) -> f64 {
    let mut total = 0.0;
    let mut count = 0.0;
    for value in values {
        total += f64::from(value);
        count += 1.0;
    }
    if count == 0.0 { 0.0 } else { total / count }
}
