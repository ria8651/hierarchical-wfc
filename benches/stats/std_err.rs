pub struct StdErr<T> {
    pub n: T,
    pub s: T,
}

impl std::fmt::Display for StdErr<f64> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let sf_index = self.s.log10().floor() as i32;

        let n = (self.n.abs() * (10f64).powi(1 - sf_index)).round() as i64;
        let s = (self.s.abs() * (10f64).powi(1 - sf_index)).round() as i64;

        let sign = if self.n.is_sign_negative() { "-" } else { "" };
        let n = format!("{}{}.{}", sign, n.div_euclid(10), n.rem_euclid(10));
        let s = format!("{}.{}", s.div_euclid(10), s.rem_euclid(10));

        write!(f, "({}pm{})e{}", n, s, sf_index)
    }
}

impl std::fmt::Debug for StdErr<f64> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let sf_index = self.s.log10().floor() as i32;

        let n = (self.n.abs() * (10f64).powi(1 - sf_index)).round() as i64;
        let s = (self.s.abs() * (10f64).powi(1 - sf_index)).round() as i64;

        let sign = if self.n.is_sign_negative() { "-" } else { "" };
        let n = format!("{}{}.{}", sign, n.div_euclid(10), n.rem_euclid(10));
        let s = format!("{}.{}", s.div_euclid(10), s.rem_euclid(10));

        write!(f, "({}pm{})e{}", n, s, sf_index)
    }
}

impl StdErr<f64> {
    /// T-Test formula for two samples:
    /// t = X_1 - X_2/S
    /// Where S is the pooled standard error:
    /// sqrt( (std_err_1)^2 + (std_err_2)^2)
    pub fn t_test(&self, other: &Self) -> f64 {
        (self.n - other.n) / (self.s * self.s + other.s * other.s).sqrt()
    }
}

// https://en.wikipedia.org/wiki/Standard_deviation#Rapid_calculation_methods
#[derive(Default)]
pub struct RollingStdErr<T> {
    pub current: T,
    pub s_1: T,
    pub s_2: T,
    pub n: usize,
}

impl RollingStdErr<f64> {
    pub fn increment(&mut self, v: f64) {
        self.current += v;
    }

    pub fn commit(&mut self) {
        self.s_1 += self.current;
        self.s_2 += self.current * self.current;
        self.current = 0.0;
        self.n += 1;
    }

    pub fn avg(&self) -> StdErr<f64> {
        if self.n == 0 {
            return StdErr::<f64> { n: 0.0, s: 0.0 };
        }

        let avg = self.s_1 / self.n as f64;
        let sigma = (self.n as f64 * self.s_2 - self.s_1 * self.s_1).sqrt() / self.n as f64;
        let std_err = sigma / (self.n as f64).sqrt();
        StdErr::<f64> { n: avg, s: std_err }
    }
}
