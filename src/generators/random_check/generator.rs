use super::adjacency::Adjacency;
use rand::distributions::WeightedIndex;
use rand::Rng;

/// A `Generator` helps generating checks for a code while respecting some global constraints.
///
/// Constraints are degrees and minimal girth. A `Generator` is consumed while generating checks
/// and need to be reset before being use to generate checks for another code.
///
/// # Example 
///
/// ```
/// use believer::generators::random_check::Generator;
/// use rand::SeedableRng;
/// use rand_chacha::ChaCha8Rng;
///
/// // Create a check generator for 10 bits.
/// let mut generator = Generator::with_n_bits(10);
///
/// // Set the parameters of the generator.
/// generator.set_maximal_bit_degree(3).set_minimal_girth(4);
///
/// // Create a random number generator
/// let mut rng = ChaCha8Rng::seed_from_u64(123);
///
/// // Generate a check
/// let first_check = generator.get_random_check(&mut rng);
///
/// // Limit the next check to be on the first 3 bits.
/// let second_check = generator.set_over_bits(vec![0, 1, 2]).get_random_check(&mut rng);
///
/// // Use relative weight for the next check.
/// let weights = (0..10).collect<Vec<f64>>();
/// let third_check = generator.set_distribution(weights).get_random_check(&mut rng);
///
/// ```
pub struct Generator {
    maximal_bit_degree: usize,
    bit_degrees: Vec<usize>,
    adjacency: Adjacency,
    active_bits: Vec<usize>,
    distribution: Vec<f64>,
}

impl Generator {
    // ***** Construction *****

    /// Creates a generator for empty code.
    pub fn new() -> Self {
        Self {
            maximal_bit_degree: 0,
            bit_degrees: Vec::new(),
            adjacency: Adjacency::new(),
            active_bits: Vec::new(),
            distribution: Vec::new(),
        }
    }

    /// Creates a generator for `n_bits` without any restriction on the checks that are going
    /// to be generate.
    pub fn with_n_bits(n_bits: usize) -> Self {
        Generator::new()
            .initialize_bit_degrees(n_bits)
            .initialize_adjacency(n_bits)
            .initialize_active_bits(n_bits)
            .initialize_distribution(n_bits)
    }

    fn initialize_bit_degrees(mut self, n_bits: usize) -> Self {
        self.bit_degrees = vec![0; n_bits];
        self
    }

    fn initialize_adjacency(mut self, n_bits: usize) -> Self {
        self.adjacency = Adjacency::with_n_bits(n_bits);
        self
    }

    fn initialize_active_bits(mut self, n_bits: usize) -> Self {
        self.active_bits = (0..n_bits).collect();
        self
    }

    fn initialize_distribution(mut self, n_bits: usize) -> Self {
        self.distribution = vec![1.0 / n_bits as f64; n_bits];
        self
    }

    // ***** Setters *****

    /// Set the minimal girth of `self`. 
    /// 
    /// The checks generated after this won't create any cycle smaller than the minimal girth.
    /// If this is set before generating the first checks, the generated checks will induce 
    /// a code with girth at least `minimal_girth`.
    pub fn set_minimal_girth(&mut self, minimal_girth: usize) -> &mut Self {
        self.adjacency.set_recursion_depth(minimal_girth / 2);
        self
    }

    /// Set the maximal bit degree of `self`. 
    /// 
    /// A given bit will not be part of any new check if it was generated in
    /// `maximal_bit_degree` previous checks.
    pub fn set_maximal_bit_degree(&mut self, degree: usize) -> &mut Self {
        self.maximal_bit_degree = degree;
        self
    }

    /// Set the generator to generate the following checks without any restriction on the bits.
    /// 
    /// # Example 
    pub fn set_over_all_bits(&mut self) -> &mut Self {
        self.active_bits = (0..self.get_n_bits()).collect();
        self
    }

    pub fn set_over_bits(&mut self, mut bits: Vec<usize>) -> &mut Self {
        bits.sort();
        bits.dedup();
        self.active_bits = bits;
        self
    }

    pub fn set_without_bits(&mut self, bits: &[usize]) -> &mut Self {
        bits.iter().for_each(|bit| {
            if let Some(index) = self.active_bits.iter().position(|b| b == bit) {
                self.active_bits.swap_remove(index);
            }
        });
        self
    }

    pub fn set_distribution(&mut self, distribution: Vec<f64>) -> &mut Self {
        if distribution.len() != self.get_n_bits() {
            panic!("wrong number of probabilities");
        }
        if distribution.iter().any(|prob| *prob < 0.0) {
            panic!("there are some negative probabilities");
        }
        self.distribution = distribution;
        self
    }

    pub fn set_uniform_distribution(&mut self) -> &mut Self {
        self.distribution = vec![1.0 / self.get_n_bits() as f64; self.get_n_bits()];
        self
    }

    pub fn reset(&mut self) {
        let n_bits = self.get_n_bits();
        self.initialize_bit_degrees(n_bits);
        self.initialize_adjacency(n_bits);
    }

    // ***** Getters *****

    /// Returns the list of bits adjacent to `bit` given the minimal girth of the generator.
    ///
    /// Two bits are adjacent if connecting them to the same check will create a cycle smaller than
    /// the minimal girth.
    pub fn get_bits_adjacent_to(&self, bit: usize) -> Vec<usize> {
        self.adjacency.get_bits_adjacent_to(bit)
    }

    /// Returns the number of bits in the code that `self` is generating checks for.
    pub fn get_n_bits(&self) -> usize {
        self.bit_degrees.len()
    }

    /// Generates a random check of degree `check_degree` using the random number generator `rng`.
    pub fn get_random_check<R: Rng>(
        &mut self,
        check_degree: usize,
        rng: &mut R
    ) -> Option<Vec<usize>> {
        let candidate_check = self.get_candidate_check(check_degree, rng);
        if Self::is_valid_check(&candidate_check) {
            self.update_from_check(&candidate_check);
            Some(candidate_check)
        } else {
            None
        }
    }

    fn get_candidate_check<R: Rng>(&self, degree: usize, rng: &mut R) -> Vec<usize> {
        let mut check = Vec::with_capacity(degree);
        for _ in 0..degree {
            self.add_random_bit_to_check(&mut check, rng);
        }
        check
    }

    fn add_random_bit_to_check<R: Rng>(&self, check: &mut Vec<usize>, rng: &mut R) {
        self.get_random_bit_generator_for_check(check)
            .add_random_bit_to_check(check, rng);
    }

    fn get_random_bit_generator_for_check(&self,check: &[usize],
    ) -> RandomBitGenerator {
        let availables = self.get_available_bits_for_check(check);
        let distribution = self.get_distribution_over(&availables);
        RandomBitGenerator{ availables, distribution }
    }

    fn get_available_bits_for_check(&self, check: &[usize]) -> Vec<usize> {
        self.active_bits
            .iter()
            .filter(|bit| self.is_available(**bit))
            .filter(|bit| self.is_not_adjacent_to_check(bit, check))
            .cloned()
            .collect()
    }

    fn is_available(&self, bit: usize) -> bool {
        self.bit_degrees[bit] < self.maximal_bit_degree
    }

    fn is_not_adjacent_to_check(&self, bit: &usize, check: &[usize]) -> bool {
        check.iter().all(|b| self.are_not_adjacent(*b, bit))
    }

    fn are_not_adjacent(&self, bit_0: usize, bit_1: &usize) -> bool {
        !self.get_bits_adjacent_to(bit_0).contains(bit_1)
    }

    fn get_distribution_over(&self, bits: &[usize]) -> Vec<f64> {
        bits
            .iter()
            .map(|bit| self.distribution[*bit])
            .collect()
    }

    fn update_from_check(&mut self, check: &[usize]) {
        self.update_degrees_from_check(check);
        self.update_adjacency_from_check(check);
    }

    fn is_valid_check(check: &[usize]) -> bool {
        // A length 0 check is useless and a length 1 check only froze a bit in the code.
        check.len() >= 2
    }

    fn update_degrees_from_check(&mut self, check: &[usize]) {
        check.iter().for_each(|bit| self.bit_degrees[*bit] += 1);
    }

    fn update_adjacency_from_check(&mut self, check: &[usize]) {
        self.adjacency.update_from_check(check)
    }
}

struct RandomBitGenerator {
    availables: Vec<usize>,
    distribution: Vec<f64>,
}

impl RandomBitGenerator {
    fn add_random_bit_to_check<R: Rng>(self, check: &mut Vec<usize>, rng: &mut R) {
        if let Ok(distribution) = WeightedIndex::new(&self.distribution) {
            check.push(self.availables[rng.sample(distribution)]);
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use rand::SeedableRng;
    use rand_chacha::ChaCha8Rng;

    #[test]
    fn doesnt_include_same_bit_twice() {
        let mut rng = ChaCha8Rng::seed_from_u64(10);

        let mut generator = Generator::with_n_bits(3);
        generator.set_maximal_bit_degree(2);

        let first_check = generator.set_over_bits(vec![0, 1]).get_random_check(2, &mut rng);
        assert_eq!(first_check, Some(vec![0, 1]));

        // Can't generate a degree 4 check over only 2 bits.
        let second_check = generator.get_random_check(4, &mut rng);
        assert_eq!(second_check, None);
    }

    #[test]
    fn doesnt_exceed_bit_maximal_degree() {
        let mut rng = ChaCha8Rng::seed_from_u64(10);

        let mut generator = Generator::with_n_bits(3);
        generator.set_maximal_bit_degree(2);

        let first_check = generator.set_without_bits(&[2]).get_random_check(2, &mut rng);
        assert_eq!(first_check, Some(vec![0, 1]));

        let second_check = generator
            .set_over_all_bits()
            .set_without_bits(&[0])
            .get_random_check(2, &mut rng);
        assert_eq!(second_check, Some(vec![1, 2]));

        // We already have checks [0,1] and [1, 2]. Degree of bit 1 is 2 and it can't
        // be included in another check.

        let third_check = generator.set_over_all_bits().get_random_check(3, &mut rng);
        assert_eq!(third_check, None);

        let fourth_check = generator.get_random_check(2, &mut rng);
        assert_eq!(fourth_check, Some(vec![0, 2]));

        // Every bit has max degree. Can't generate anymore check
        assert_eq!(generator.get_random_check(1, &mut rng), None);
    }

    #[test]
    fn doesnt_create_cycle_smaller_than_minimal_girth() {
        let mut rng = ChaCha8Rng::seed_from_u64(10);

        // Minimal girth 6
        let mut generator = Generator::with_n_bits(3);
        generator.set_maximal_bit_degree(2).set_minimal_girth(6);

        let first_check = generator.get_random_check(3, &mut rng);
        assert_eq!(first_check, Some(vec![0, 1, 2]));

        // Any check of degree 2 will create a 4-cycle.
        let second_check = generator.get_random_check(2, &mut rng);
        assert_eq!(second_check, None);

        // A degree 1 check will not create a 4-cycle.
        let third_check = generator.get_random_check(1, &mut rng);
        assert_eq!(third_check.is_some(), true);

        // Minimal girth 8
        let mut generator = Generator::with_n_bits(5);
        generator.set_maximal_bit_degree(2).set_minimal_girth(8);

        let first_check = generator.set_over_bits(vec![0, 1, 2]).get_random_check(3, &mut rng);
        assert_eq!(first_check, Some(vec![0, 1, 2]));

        let second_check = generator.set_over_bits(vec![2, 3]).get_random_check(2, &mut rng);
        assert_eq!(second_check, Some(vec![2, 3]));

        // A check over [0, 3] will create a 6-cycle.
        let third_check = generator.set_over_bits(vec![0, 3]).get_random_check(2, &mut rng);
        assert_eq!(third_check, None);

        // Possible checks are [0, 4] or [3, 4]
        let fourth_check = generator.set_over_bits(vec![0, 3, 4]).get_random_check(2, &mut rng);
        assert_eq!(fourth_check.clone().unwrap().contains(&4), true);
        assert_eq!(fourth_check.unwrap().len(), 2);
    }

    #[test]
    fn generate_bit_according_to_distribution() {
        let mut rng = ChaCha8Rng::seed_from_u64(10);

        let mut generator = Generator::with_n_bits(5);
        generator
            .set_maximal_bit_degree(2)
            .set_distribution(vec![0.25, 0.25, 0.0, 0.25, 0.25])
            .set_over_bits(vec![0, 1, 2]);

        // Can't generate 3 bits from this distribution over the first 3.
        assert_eq!(generator.get_random_check(3, &mut rng), None);
        assert_eq!(generator.get_random_check(2, &mut rng), Some(vec![0, 1]));
        assert_eq!(generator.get_random_check(2, &mut rng), Some(vec![0, 1]));

        // Degree of the first 2 bits is 2.
        assert_eq!(generator.get_random_check(2, &mut rng), None);

        generator.set_over_all_bits();
        assert_eq!(generator.get_random_check(2, &mut rng), Some(vec![3, 4]));

        // Can't pick a degree 3 check because probability of bit 2 is 0.
        assert_eq!(generator.get_random_check(3, &mut rng), None);

        // Reset distribution.
        generator.set_uniform_distribution();
        assert_eq!(generator.get_random_check(3, &mut rng), Some(vec![2, 3, 4]));
    }
}
