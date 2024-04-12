use core::fmt;

pub use oblast_demo::Fr;

/// OBJECTIVEs
/// 1. Implement a struct Polynomial that represents a polynomial. [Done]
/// 2. Implement the Display trait for Polynomial so that we can print it out. [Done]
/// 3. Implement a method evaluate for Polynomial that evaluates the polynomial at a given point.
/// 4. Implement a method to create a polynomial from a list of coefficients.[Done]

#[derive(Debug, Clone)]
pub struct Polynomial {
    pub coefficients: Vec<Fr>,
}

impl fmt::Display for Polynomial {
    // Create fmt to formart the vec of coefficients
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut s = String::new();
        for (i, c) in self.coefficients.iter().enumerate() {
            if i == 0 {
                s.push_str(&format!("{:?}", c));
            } else {
                s.push_str(&format!(" + {:?}x^{:?}", c, i));
            }
        }
        write!(f, "{}", s)
    }
}

impl Polynomial {
    pub fn from_coefficients(coefficients: Vec<Fr>) -> Self {
        Self { coefficients }
    }

    pub fn evalaute(&self, x: Fr) -> Fr {
        let mut sum = self.coefficients[0].clone();
        let mut variable = x.clone();

        for i in 1..self.coefficients.len() {
            sum += self.coefficients[i] * variable;
            variable *= variable;
        }

        sum
    }


}


#[cfg(test)]
mod tests {
    use std::fmt::Debug;

    use super::*;

    #[test]
    fn evaluate_test() {
        let polynomial = Polynomial::from_coefficients(vec![Fr::from_u64(1), Fr::from_u64(3), Fr::from_u64(2)]);
        let eval = polynomial.evalaute(Fr::from_u64(2));

        assert_eq!(eval, Fr::from_u64(15));
    }
}
