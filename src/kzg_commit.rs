use oblast_demo::{curve_order, verify_pairings, Scalar, P1, P2, Fr};
use num_bigint::BigUint;
use rand::prelude::*;

use crate::polynomial; // Important for generating Tau (during power of tau)

/// CURVE: BLS12-381 (G1, G2, GT)




/// [0, 1, 2, 3, 4, 5, 6] mod 7
/// How many point exist in a elliptic curve group? --> `curve_order` 255bits
/// e(input1, input2) -> output [bilinear pairing]
/// `Scalar` is a wrapper around `BigUint` that ensures that the value is less than the curve order.
/// `P1` is a point in G1, `P2` is a point in G2, `Fr` is a scalar.





/// Objective:
/// 1. Define need data-stuctures for KZG (PP -> Public parameter, KZG, Commitment, Opening)
/// 2. Implement a method to generate a public parameter for KZG
/// 3. Implement a method to commit to a polynomial
/// 4. Implement a method to open a polynomial
/// 5. Implement a method for computing qoutent polynomial 
/// 6. Implement a method for verifying commitment proofs
/// 7. Test the implementation



#[derive(Clone, Debug, PartialEq)]
pub struct PP {
    /// Powers of Tau for P1 
    pub points_in_g1: Vec<P1>,
    /// Powers of Tau for P2
    pub point_in_g2: P2 // g2 ^ tau
}


#[derive(Clone, Debug, PartialEq)]
pub  struct KZG {
    /// Shared Referenced String
    pub public_parameter: PP
}

#[derive(Debug)]
pub struct Commitment<'a> {
    /// The commitment point 
    pub element: P1,
    /// The Polynomial committed to 
    pub polynomial: &'a polynomial::Polynomial,
    /// Public parameter used during the commitment process
    pub public_parameter: &'a PP,
}

#[derive(Debug)]
pub struct Opening {
    /// The value of the polynomial at the point
    pub value: Fr,
    /// This is the proof of an Evaluation
    pub proof: P1,
}


// ======================
// CUSTOM DEFINED ERROR;
// ======================
#[derive(Debug)]
pub enum KZGErrors {
    SecretMustBeLessThanTheOrderOfTheGroup
}


impl KZG {
    /// creating a new KZG instance (randomlly computing tau and generating the public parameter)
    fn new(tau: &[u8; 32], degree: usize) -> Result<KZG, KZGErrors> {
        KZG::setup_internal(tau, degree)
    }

    /// this is the random generation function
    fn new_rand(degree: usize) -> Result<KZG, KZGErrors> {
        let mut rng = thread_rng();

        let mut secret = [0u8; 32];
        rng.fill_bytes(&mut secret);

        let mut s = BigUint::from_bytes_be(&secret);

        let modulus :BigUint = curve_order();
        while s >= modulus {
            rng.fill_bytes(&mut secret);
            s = BigUint::from_bytes_be(&secret);
        }


        KZG::setup_internal(&secret, degree)
    }

    /// this function takes in tau and computes the powers of tau
    fn setup_internal(tau: &[u8; 32], degree: usize) -> Result<KZG, KZGErrors> {
        let modulus = curve_order();
        let bytes_tau = BigUint::from_bytes_be(tau);


        if bytes_tau > modulus {
            return Err(KZGErrors::SecretMustBeLessThanTheOrderOfTheGroup);
        }

        let mut points_in_g1 = vec![];

        // obtaining the generator in the first group (this is the cyclic group)
        let g1 = P1::generator();

        // obtaining the "power of tau" (a part of the public parameter)
        for i in 0..=degree {
            let i_as_bigint = BigUint::from_slice(&[i as u32]);
            let s_i_as_bigint = bytes_tau.modpow(&i_as_bigint, &modulus);

            let mut s_i_bytes = vec![0u8; 32];
            let raw_bytes = s_i_as_bigint.to_bytes_be();
            s_i_bytes[32 - raw_bytes.len()..].copy_from_slice(&raw_bytes);
            let s_i_scalar = Scalar::from_fr_bytes(&s_i_bytes);

            let result = s_i_scalar * g1;
            points_in_g1.push(result);
        }


        let scalar = Scalar::from_fr_bytes(tau);
        let result_in_g2 = scalar * P2::generator();

        let public_parameter = PP {
            points_in_g1,
            point_in_g2: result_in_g2,
        };

        Ok(
            KZG {
                public_parameter
            }
        )
    }

    /// this function takes in a public parameter and a polynomial and returns a commitment, this commitment is this struct is a point on the G1 curve
    pub fn commit<'a>(
        public_parameter: &'a PP,
        polynomial: &'a polynomial::Polynomial,
    ) -> Result<Commitment<'a>, KZGErrors> {
        let basis = &public_parameter.points_in_g1;
        let coefficients = &polynomial.coefficients;

        let mut result = P1::default();
        for (coefficient, element) in coefficients.iter().zip(basis.iter()) {
            let term = *coefficient * *element;
            result = result + term;
        }

        Ok(Commitment {
            element: result,
            polynomial,
            public_parameter: &public_parameter,
        })
    }
}


impl<'a> Commitment<'a> {
    /// this function takes in a point and returns an opening, this opening is a struct that contains the value of the polynomial at the point and the proof of the evaluation  
    pub fn open_at(self: &Self, point: Fr) -> Result<Opening, KZGErrors> {
        let result = self.polynomial.evalaute(point);

        // divisor `s - x` for `f(x) = y`
        let divisor_coefficients = vec![-point, Fr::from_u64(1)];
        let divisor = polynomial::Polynomial::from_coefficients(divisor_coefficients);
        let quotient_polynomial = compute_quotient(self.polynomial, &divisor);

        let commitment = KZG::commit(self.public_parameter, &quotient_polynomial)?;

        Ok(Opening {
            value: result,
            proof: commitment.element,
        })
    }
}


// ===================================
// FREE FUNCTIONS
// ===================================
/// This is a simple function for dividing a polynomial and returning the q
fn compute_quotient(
    dividend: &polynomial::Polynomial,
    divisor: &polynomial::Polynomial,
) -> polynomial::Polynomial {
    let mut dividend = dividend.coefficients.clone();
    let mut coefficients = vec![];

    let mut dividend_pos = dividend.len() - 1;
    let divisor_pos = divisor.coefficients.len() - 1;
    let mut difference = dividend_pos as isize - divisor_pos as isize;

    while difference >= 0 {
        let term_quotient = dividend[dividend_pos] / divisor.coefficients[divisor_pos];
        coefficients.push(term_quotient);

        for i in (0..=divisor_pos).rev() {
            let difference = difference as usize;
            let x = divisor.coefficients[i];
            let y = x * term_quotient;
            let z = dividend[difference + i];
            dividend[difference + i] = z - y;
        }

        dividend_pos -= 1;
        difference -= 1;
    }

    coefficients.reverse();
    polynomial::Polynomial { coefficients }
}

impl Opening {
    /// this function takes in an input and a commitment and returns a boolean value, this boolean value is true if the proof is valid and false otherwise
    pub fn verify(&self, input: &Fr, commitment: &Commitment) -> bool {
        // Compute [f(s) - y]_1 for LHS
        let y_p1 = self.value * P1::generator();
        let commitment_minus_y = commitment.element + -y_p1;

        // Compute [s - z]_2 for RHS
        let z_p2 = *input * P2::generator();
        let s_minus_z = commitment.public_parameter.point_in_g2 + -z_p2;

        verify_pairings(commitment_minus_y, P2::generator(), self.proof, s_minus_z)
    }
}











/// This this is a sample test from Ethereum SPECS for EIP4844
#[cfg(test)]
mod tests {
    use crate::polynomial::Polynomial;

    use super::*;

    #[test]
    fn test_setup() {
        let tau = [34u8; 32];
        let degree = 29;


        let kzg = KZG::new(&tau, degree).unwrap();
        println!("This is KZG -> {:?}", kzg);
        assert_eq!(kzg.public_parameter.points_in_g1.len(), degree + 1);
    }





    #[test]
    fn test_opening() {
        // computed from python reference: https://github.com/ethereum/research/blob/master/kzg_data_availability/kzg_proofs.py
        let test_cases = vec![
            (
                "0000000000000000000000000000000000000000000000000000000000000000",
                vec![0],
                0,
                "c00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000",
                "c00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000",
            ),
            (
                "0000000000000000000000000000000000000000000000000000000000000000",
                vec![11],
                11,
                "80fd75ebcc0a21649e3177bcce15426da0e4f25d6828fbf4038d4d7ed3bd4421de3ef61d70f794687b12b2d571971a55",
                "c00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000",
            ),
            (
                "0000000000000000000000000000000000000000000000000000000000000000",
                vec![0, 1],
                15,
                "c00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000",
                "97f1d3a73197d7942695638c4fa9ac0fc3688c4f9774b905a14e3a3f171bac586c55e83ff97a1aeffb3af00adb22c6bb",
            ),
            (
                "0000000000000000000000000000000000000000000000000000000000000000",
                vec![1, 12],
                181,
                "97f1d3a73197d7942695638c4fa9ac0fc3688c4f9774b905a14e3a3f171bac586c55e83ff97a1aeffb3af00adb22c6bb",
                "8345dd80ffef0eaec8920e39ebb7f5e9ae9c1d6179e9129b705923df7830c67f3690cbc48649d4079eadf5397339580c",
            ),
            (
                "0000000000000000000000000000000000000000000000000000000000000000",
                vec![1, 2, 2],
                481,
                "97f1d3a73197d7942695638c4fa9ac0fc3688c4f9774b905a14e3a3f171bac586c55e83ff97a1aeffb3af00adb22c6bb",
                "a72841987e4f219d54f2b6a9eac5fe6e78704644753c3579e776a3691bc123743f8c63770ed0f72a71e9e964dbf58f43",
            ),
            (
                "0000000000000000000000000000000000000000000000000000000000000000",
                vec![1, 2, 3, 4, 7, 7, 7, 7, 13, 13, 13, 13, 13, 13, 13, 13],
                6099236329206434206,
                "97f1d3a73197d7942695638c4fa9ac0fc3688c4f9774b905a14e3a3f171bac586c55e83ff97a1aeffb3af00adb22c6bb",
                "95c2663b029a933ca94f346061b52dfc85da11386c9aaffe2b604a00589299c10b0855f90c5f7db31cc1cc45353dc948",
            ),
            (
                "0b598c0727a94e556b8c1dcb64af40daea6971901b5dcb8b49da2fe2b533a52e",
                vec![0],
                0,
                "c00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000",
                "c00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000",
            ),
            (
                "0b598c0727a94e556b8c1dcb64af40daea6971901b5dcb8b49da2fe2b533a52e",
                vec![11],
                11,
                "80fd75ebcc0a21649e3177bcce15426da0e4f25d6828fbf4038d4d7ed3bd4421de3ef61d70f794687b12b2d571971a55",
                "c00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000",
            ),
            (
                "0b598c0727a94e556b8c1dcb64af40daea6971901b5dcb8b49da2fe2b533a52e",
                vec![0, 1],
                15,
                "b6464852dee959d00049ce3630a863d5226309fc9cdcb50d991b571a4e8b2f55c61955045918ab4bd6c0460a01fedfe0",
                "97f1d3a73197d7942695638c4fa9ac0fc3688c4f9774b905a14e3a3f171bac586c55e83ff97a1aeffb3af00adb22c6bb",
            ),
            (
                "0b598c0727a94e556b8c1dcb64af40daea6971901b5dcb8b49da2fe2b533a52e",
                vec![1, 12],
                181,
                "adea87ebbba6c937d96ea9bac45a5de282b17bce08e40ab6ed358e2eedda5a0e667a9a744d1369b6e7ffe049686261de",
                "8345dd80ffef0eaec8920e39ebb7f5e9ae9c1d6179e9129b705923df7830c67f3690cbc48649d4079eadf5397339580c",
            ),
            (
                "0b598c0727a94e556b8c1dcb64af40daea6971901b5dcb8b49da2fe2b533a52e",
                vec![1, 2, 2],
                481,
                "b3e43da9f207cb9d717f85d40b967a28254b22bb6269b551aed50444eb1aed7f93a2b519acd7076e56451dc084389323",
                "b8cea544c0d68bf429533df6126a3f9a3ce9027595df4e7fc1e00a368f8b92690251434e51a9b53b35e8e9677960e0b1",
            ),
            (
                "0b598c0727a94e556b8c1dcb64af40daea6971901b5dcb8b49da2fe2b533a52e",
                vec![1, 2, 3, 4, 7, 7, 7, 7, 13, 13, 13, 13, 13, 13, 13, 13],
                6099236329206434206,
                "970d3aa5cad4492adb0c87c1f9ee4a82e48a59777d66868827080c145e4562995348af9a486b59f7bdf62a7c25c7159f",
                "b37b9247ff4965586a6e6bb0c5634e34865c233c5c2efc123410fa9f536da2d258c816d3b2db7a3c9c54311837fea7ac",
            ),
            (
                "57a29351ad759e70ac84de21c4a5a54780b46b1a7cfc5bfa033e3b9321562bce",
                vec![0],
                0,
                "c00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000",
                "c00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000",
            ),
            (
                "57a29351ad759e70ac84de21c4a5a54780b46b1a7cfc5bfa033e3b9321562bce",
                vec![11],
                11,
                "80fd75ebcc0a21649e3177bcce15426da0e4f25d6828fbf4038d4d7ed3bd4421de3ef61d70f794687b12b2d571971a55",
                "c00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000",
            ),
            (
                "57a29351ad759e70ac84de21c4a5a54780b46b1a7cfc5bfa033e3b9321562bce",
                vec![0, 1],
                15,
                "94976e86763f440d1338d7c17d181c027630dc39a1d648068683d228300b1085d0c4fbfd9f6f308cda71fdd641834a36",
                "97f1d3a73197d7942695638c4fa9ac0fc3688c4f9774b905a14e3a3f171bac586c55e83ff97a1aeffb3af00adb22c6bb",
            ),
            (
                "57a29351ad759e70ac84de21c4a5a54780b46b1a7cfc5bfa033e3b9321562bce",
                vec![1, 12],
                181,
                "a2dffe3cfef260770472215a66689c0ad35d2fd5868ea369e1a65c47c1cabdb1786a8e5763021b0cac33f458650e80ce",
                "8345dd80ffef0eaec8920e39ebb7f5e9ae9c1d6179e9129b705923df7830c67f3690cbc48649d4079eadf5397339580c",
            ),
            (
                "57a29351ad759e70ac84de21c4a5a54780b46b1a7cfc5bfa033e3b9321562bce",
                vec![1, 2, 2],
                481,
                "a8372e96e8db620e5a5a359f884aea597f358ba9b54d3bf36c712e241dc612e2a7fa81efe3159b2eff19c84b0b7f31f5",
                "acb40f1a984eba565dc9025284fc32f58e01f4bc1af92edbe8114151057998c45da684e50563a2a0a2660d374d851a2f",
            ),
            (
                "57a29351ad759e70ac84de21c4a5a54780b46b1a7cfc5bfa033e3b9321562bce",
                vec![1, 2, 3, 4, 7, 7, 7, 7, 13, 13, 13, 13, 13, 13, 13, 13],
                6099236329206434206,
                "81cdc95341621862ebf968daf2760c5412beecb06d272d276a007e1a9c0355f2b053c7bb3e1569366ab7e1b414c5af2e",
                "89e2eb1c44cc5ad3337562570c9940737a1e006a0148f7982c8f3c99bf6484cba0b86edc082b5b90da4190b588c3a3bb",
            ),
        ];

        let point = Fr::from_u64(15);

        for (secret_hex, polynomial, value, expected_commitment_hex, expected_proof_hex) in
        test_cases
        {
            let secret = hex::decode(secret_hex).unwrap();
            let coefficients = polynomial.into_iter().map(Fr::from_u64).collect::<Vec<_>>();

            let degree = coefficients.len();

            let secret = secret.as_slice().try_into().unwrap();
            let setup = KZG::new(secret, degree).unwrap().public_parameter;

            let polynomial = Polynomial::from_coefficients(coefficients);

            let commitment = KZG::commit(&setup, &polynomial).unwrap();

            let opening = commitment.open_at(point).unwrap();

            // does evaluation match?
            assert_eq!(opening.value.as_u64(), value);

            // does commitment match?
            let commitment_serialization = commitment.element.compress();
            let expected_commitment_serialization = hex::decode(expected_commitment_hex).unwrap();
            assert_eq!(commitment_serialization, expected_commitment_serialization);

            // does proof match?
            let proof_serialization = opening.proof.compress();
            let expected_proof_serialization = hex::decode(expected_proof_hex).unwrap();
            assert_eq!(proof_serialization, expected_proof_serialization);

            // does the proof verify?
            assert!(opening.verify(&point, &commitment));
        }
    }


}