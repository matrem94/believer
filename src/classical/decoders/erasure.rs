//! A classical erasure decoder.

use super::{Decoder, DecodingResult};
use crate::ErasureResult;
use crate::Ressources;
use crate::ParityCheckMatrix;
use rand::Rng;

/// Decoder for classical erasure channel.
///
/// # Example
///
/// ```
/// # use believer::*;
/// let code = ParityCheckMatrix::with_n_bits(3).with_checks(vec![vec![0, 1], vec![1, 2]]);
/// let decoder = ErasureDecoder::with_prob(0.25).for_code(code);
/// decoder.decode(&decoder.get_random_error());
/// ```
#[derive(Debug)]
pub struct ErasureDecoder {
    code: ParityCheckMatrix,
    erasure_prob: f64,
    ressources: Ressources,
}

impl ErasureDecoder {
    /// Creates an erasure decoder.
    ///
    /// # Panic
    ///
    /// Panics if `erasure_prob` is not between 0.0 and 1.0.
    pub fn with_prob(erasure_prob: f64) -> Self {
        if erasure_prob < 0.0 || erasure_prob > 1.0 {
            panic!("invalid probability");
        }

        Self {
            erasure_prob,
            code: ParityCheckMatrix::new(),
            ressources: Ressources{
                rank_mtx: None,
                sum_vec: None,
            },
            
        }
    }

    fn next_bit_is_erased<R: Rng>(&self, rng: &mut R) -> bool {
        rng.gen::<f64>() < self.erasure_prob
    }
}

impl Decoder for ErasureDecoder {
    type Error = Vec<usize>; // Positions of erased bits.
    type Result = ErasureResult;
    type Code = ParityCheckMatrix;

    fn for_code(mut self, code: Self::Code) -> Self {

        let rank_mtx = Some(code.tmp_rank_pcm());
        let sum_vec = Some(Vec::with_capacity(code.get_n_bits()));

        self.ressources = Ressources{
            rank_mtx,
            sum_vec,
        };

        self.code = code;

        self
    }

    fn take_code(&mut self) -> Self::Code {
        std::mem::replace(&mut self.code, ParityCheckMatrix::new())
    }

    // An erasure error can be corrected if there is no information in the erased submatrix. That
    // is, the number of erased bits is equal to the rank of the parity check matrix restricted to
    // the erased bit columns.
    fn decode(&mut self, error: &Self::Error) -> Self::Result {

        let mut rank_mtx = self.ressources.rank_mtx.take().unwrap();
        let mut sum_vec = self.ressources.sum_vec.take().unwrap();

        let erased_parity_check = self.code.keep(error);

        let erased_rank = erased_parity_check.rank_mut(&mut rank_mtx, &mut sum_vec);

        self.ressources.rank_mtx = Some(rank_mtx);
        self.ressources.sum_vec= Some(sum_vec);

        if error.len() - erased_rank == 0 {
            ErasureResult::Success
        } else {
            ErasureResult::Failure
        }
    }

    // Erase random bits with given probability.
    fn get_random_error_with_rng<R: Rng>(&self, rng: &mut R) -> Self::Error {
        (0..self.code.get_n_bits())
            .filter(|_| self.next_bit_is_erased(rng))
            .collect()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn repetition_code() {
        let code = ParityCheckMatrix::with_n_bits(3).with_checks(vec![vec![0, 1], vec![1, 2]]);
        let mut decoder = ErasureDecoder::with_prob(0.2).for_code(code);

        assert_eq!(decoder.decode(&vec![]), ErasureResult::Success);
        for i in 0..=2 {
            assert_eq!(decoder.decode(&vec![i]), ErasureResult::Success);
            for j in (i + 1)..=2 {
                assert_eq!(decoder.decode(&vec![i, j]), ErasureResult::Success);
            }
        }
        assert_eq!(decoder.decode(&vec![0, 1, 2]), ErasureResult::Failure);
    }

    #[test]
    fn hamming_code() {
        let code = ParityCheckMatrix::with_n_bits(7).with_checks(vec![
            vec![0, 1, 2, 4],
            vec![0, 1, 3, 5],
            vec![0, 2, 3, 6],
        ]);
        let mut decoder = ErasureDecoder::with_prob(0.25).for_code(code);

        assert_eq!(decoder.decode(&vec![]), ErasureResult::Success);
        for i in 0..=6 {
            assert_eq!(decoder.decode(&vec![i]), ErasureResult::Success);
            for j in (i + 1)..=6 {
                assert_eq!(decoder.decode(&vec![i, j]), ErasureResult::Success);
            }
        }
        assert_eq!(decoder.decode(&vec![0, 1, 2]), ErasureResult::Success);
        assert_eq!(decoder.decode(&vec![2, 4, 5]), ErasureResult::Success);
        assert_eq!(decoder.decode(&vec![0, 1, 4]), ErasureResult::Success);
        assert_eq!(decoder.decode(&vec![3, 4, 5]), ErasureResult::Success);

        assert_eq!(decoder.decode(&vec![2, 4, 6]), ErasureResult::Failure);
        assert_eq!(decoder.decode(&vec![1, 2, 3]), ErasureResult::Failure);
        assert_eq!(decoder.decode(&vec![0, 3, 4]), ErasureResult::Failure);
        assert_eq!(decoder.decode(&vec![0, 2, 5]), ErasureResult::Failure);

        assert_eq!(
            decoder.decode(&vec![0, 1, 2, 3, 4, 5, 6]),
            ErasureResult::Failure
        );
    }
}