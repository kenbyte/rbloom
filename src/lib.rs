use bitline::BitLine;
use pyo3::prelude::*;

#[pyclass]
struct Bloom {
    filter: BitLine,
    k: u64,
    hash_func: Option<PyObject>,
}

#[pymethods]
impl Bloom {
    #[new]
    fn new(
        expected_items: u64,
        false_positive_rate: f64,
        hash_func: Option<&PyAny>,
    ) -> PyResult<Self> {
        // Check the inputs
        if let Some(hash_func) = hash_func {
            if !hash_func.is_callable() {
                return Err(pyo3::exceptions::PyTypeError::new_err(
                    "hash_func must be callable",
                ));
            }
        }
        if false_positive_rate <= 0.0 || false_positive_rate >= 1.0 {
            return Err(pyo3::exceptions::PyValueError::new_err(
                "false_positive_rate must be between 0 and 1",
            ));
        }
        if expected_items == 0 {
            return Err(pyo3::exceptions::PyValueError::new_err(
                "expected_items must be greater than 0",
            ));
        }

        // Calculate the parameters for the filter
        let size_in_bits =
            -1.0 * (expected_items as f64) * false_positive_rate.ln() / 2.0f64.ln().powi(2);
        let k = (size_in_bits / expected_items as f64) * 2.0f64.ln();

        // Create the filter
        Ok(Bloom {
            filter: BitLine::new(size_in_bits as u64)?,
            k: k as u64,
            hash_func: match hash_func {
                Some(hash_func) => Some(hash_func.to_object(hash_func.py())),
                None => None,
            },
        })
    }

    fn size_in_bits(&self) -> u64 {
        self.filter.len()
    }

    fn add(&mut self, o: &PyAny) -> PyResult<()> {
        let hash = hash(o, &self.hash_func)?;
        for index in mlcg::generate_indexes(hash, self.k, self.filter.len()) {
            self.filter.set(index);
        }
        Ok(())
    }

    fn __contains__(&self, o: &PyAny) -> PyResult<bool> {
        let hash = hash(o, &self.hash_func)?;
        for index in mlcg::generate_indexes(hash, self.k, self.filter.len()) {
            if !self.filter.get(index) {
                return Ok(false);
            }
        }
        Ok(true)
    }

    fn __or__(&self, other: &Bloom) -> PyResult<Bloom> {
        check_compatible(self, other)?;
        Ok(Bloom {
            filter: self.filter.clone() | other.filter.clone(),
            k: self.k,
            hash_func: self.hash_func.clone(),
        })
    }

    fn __ior__(&mut self, other: &Bloom) -> PyResult<()> {
        check_compatible(self, other)?;
        self.filter |= other.filter.clone();
        Ok(())
    }

    fn __and__(&self, other: &Bloom) -> PyResult<Bloom> {
        check_compatible(self, other)?;
        Ok(Bloom {
            filter: self.filter.clone() & other.filter.clone(),
            k: self.k,
            hash_func: self.hash_func.clone(),
        })
    }

    fn __iand__(&mut self, other: &Bloom) -> PyResult<()> {
        check_compatible(self, other)?;
        self.filter &= other.filter.clone();
        Ok(())
    }

    fn clear(&mut self) {
        self.filter.clear();
    }

    fn copy(&self) -> Bloom {
        Bloom {
            filter: self.filter.clone(),
            k: self.k,
            hash_func: self.hash_func.clone(),
        }
    }
}

/// This is a primitive BitVec-like structure that uses a Vec<u8> as
/// the backing store; it exists here to avoid the need for a dependency
/// on bitvec and to act as a container around all the bit manipulation.
/// Indexing is done using u64 to avoid address space issues on 32-bit
/// systems, which would otherwise limit the size to 2^32 bits (512MB).
mod bitline {
    use pyo3::prelude::*;

    pub struct BitLine {
        bits: Vec<u8>,
    }

    impl BitLine {
        pub fn new(size_in_bits: u64) -> PyResult<Self> {
            let (q, r) = (size_in_bits / 8, size_in_bits % 8);
            let size = if r == 0 { q } else { q + 1 };
            Ok(BitLine {
                bits: vec![0; size.try_into()?],
            })
        }

        /// Make sure that index is less than len when calling this!
        pub fn set(&mut self, index: u64) {
            let (q, r) = (index / 8, index % 8);
            self.bits[q as usize] |= 1 << r;
        }

        /// Make sure that index is less than len when calling this!
        pub fn get(&self, index: u64) -> bool {
            let (q, r) = (index / 8, index % 8);
            self.bits[q as usize] & (1 << r) != 0
        }

        /// Returns the number of bits in the BitLine
        pub fn len(&self) -> u64 {
            self.bits.len() as u64 * 8
        }

        pub fn clear(&mut self) {
            for i in 0..self.bits.len() {
                self.bits[i] = 0;
            }
        }
    }

    impl Clone for BitLine {
        fn clone(&self) -> Self {
            BitLine {
                bits: self.bits.clone(),
            }
        }
    }

    impl std::ops::BitAnd for BitLine {
        type Output = Self;

        fn bitand(self, rhs: Self) -> Self::Output {
            let mut result = self.clone();
            for i in 0..self.bits.len() {
                result.bits[i] &= rhs.bits[i];
            }
            result
        }
    }

    impl std::ops::BitAndAssign for BitLine {
        fn bitand_assign(&mut self, rhs: Self) {
            for i in 0..self.bits.len() {
                self.bits[i] &= rhs.bits[i];
            }
        }
    }

    impl std::ops::BitOr for BitLine {
        type Output = Self;

        fn bitor(self, rhs: Self) -> Self::Output {
            let mut result = self.clone();
            for i in 0..self.bits.len() {
                result.bits[i] |= rhs.bits[i];
            }
            result
        }
    }

    impl std::ops::BitOrAssign for BitLine {
        fn bitor_assign(&mut self, rhs: Self) {
            for i in 0..self.bits.len() {
                self.bits[i] |= rhs.bits[i];
            }
        }
    }
}

/// This implements a multiplicative linear congruential generator that is
/// used to distribute entropy from the hash over multiple ints.
mod mlcg {
    pub struct Random {
        state: u128,
    }

    impl Iterator for Random {
        type Item = u64;

        fn next(&mut self) -> Option<Self::Item> {
            self.state = self
                .state
                .wrapping_mul(25096281518912105342191851917838718629);
            Some((self.state >> 32) as Self::Item)
        }
    }

    pub fn distribute_entropy(hash: i128) -> Random {
        Random {
            state: hash as u128,
        }
    }

    pub fn generate_indexes(hash: i128, k: u64, len: u64) -> impl Iterator<Item = u64> {
        distribute_entropy(hash)
            .take(k as usize)
            .map(move |x: u64| x % len)
    }
}

fn hash(o: &PyAny, hash_func: &Option<PyObject>) -> PyResult<i128> {
    match hash_func {
        Some(hash_func) => {
            let hash = hash_func.call1(o.py(), (o,))?;
            Ok(hash.extract(o.py())?)
        }
        None => Ok(o.hash()? as i128),
    }
}

fn check_compatible(a: &Bloom, b: &Bloom) -> PyResult<()> {
    if a.k != b.k || a.filter.len() != b.filter.len() {
        return Err(pyo3::exceptions::PyValueError::new_err(
            "size or max false positive rate must be the same",
        ));
    }

    // now only the hash function can be different
    let mut works = true;
    match a.hash_func {
        Some(ref hash_func) => match b.hash_func {
            Some(ref hash_func2) => {
                works &= hash_func.is(hash_func2);
            }
            None => {
                works = false;
            }
        },
        None => {
            works &= b.hash_func.is_none();
        }
    }
    match works {
        true => Ok(()),
        false => Err(pyo3::exceptions::PyValueError::new_err(
            "Bloom filters must have the same hash function",
        )),
    }
}

#[pymodule]
fn rbloom(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<Bloom>()?;
    Ok(())
}
