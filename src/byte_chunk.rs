#[cfg(target_arch = "x86_64")]
use core::arch::x86_64;

// Which type to actually use at build time.
#[cfg(all(feature = "simd", target_arch = "x86_64"))]
pub(crate) type Chunk = x86_64::__m128i;
#[cfg(any(not(feature = "simd"), not(any(target_arch = "x86_64"))))]
pub(crate) type Chunk = usize;

/// Interface for working with chunks of bytes at a time, providing the
/// operations needed for the functionality in str_utils.
pub(crate) trait ByteChunk: Copy + Clone {
    /// Size of the chunk in bytes.
    const SIZE: usize;

    /// Maximum number of iterations the chunk can accumulate
    /// before sum_bytes() becomes inaccurate.
    const MAX_ACC: usize;

    /// Creates a new chunk with all bytes set to zero.
    fn zero() -> Self;

    /// Creates a new chunk with all bytes set to n.
    fn splat(n: u8) -> Self;

    /// Returns whether all bytes are zero or not.
    fn is_zero(&self) -> bool;

    /// Shifts bytes back lexographically by n bytes.
    fn shift_back_lex(&self, n: usize) -> Self;

    /// Shifts bits to the right by n bits.
    fn shr(&self, n: usize) -> Self;

    /// Compares bytes for equality with the given byte.
    ///
    /// Bytes that are equal are set to 1, bytes that are not
    /// are set to 0.
    fn cmp_eq_byte(&self, byte: u8) -> Self;

    /// Compares bytes to see if they're in the non-inclusive range (a, b),
    /// where a < b <= 127.
    ///
    /// Bytes in the range are set to 1, bytes not in the range are set to 0.
    fn bytes_between_127(&self, a: u8, b: u8) -> Self;

    /// Performs a bitwise and on two chunks.
    fn bitand(&self, other: Self) -> Self;

    /// Adds the bytes of two chunks together.
    fn add(&self, other: Self) -> Self;

    /// Subtracts other's bytes from this chunk.
    fn sub(&self, other: Self) -> Self;

    /// Increments the nth-from-last lexographic byte by 1.
    fn inc_nth_from_end_lex_byte(&self, n: usize) -> Self;

    /// Decrements the last lexographic byte by 1.
    fn dec_last_lex_byte(&self) -> Self;

    /// Returns the sum of all bytes in the chunk.
    fn sum_bytes(&self) -> usize;
}

impl ByteChunk for usize {
    const SIZE: usize = core::mem::size_of::<usize>();
    const MAX_ACC: usize = (256 / core::mem::size_of::<usize>()) - 1;

    #[inline(always)]
    fn zero() -> Self {
        0
    }

    #[inline(always)]
    fn splat(n: u8) -> Self {
        const ONES: usize = core::usize::MAX / 0xFF;
        ONES * n as usize
    }

    #[inline(always)]
    fn is_zero(&self) -> bool {
        *self == 0
    }

    #[inline(always)]
    fn shift_back_lex(&self, n: usize) -> Self {
        if cfg!(target_endian = "little") {
            *self >> (n * 8)
        } else {
            *self << (n * 8)
        }
    }

    #[inline(always)]
    fn shr(&self, n: usize) -> Self {
        *self >> n
    }

    #[inline(always)]
    fn cmp_eq_byte(&self, byte: u8) -> Self {
        const ONES: usize = core::usize::MAX / 0xFF;
        const ONES_HIGH: usize = ONES << 7;
        let word = *self ^ (byte as usize * ONES);
        (!(((word & !ONES_HIGH) + !ONES_HIGH) | word) & ONES_HIGH) >> 7
    }

    #[inline(always)]
    fn bytes_between_127(&self, a: u8, b: u8) -> Self {
        const ONES: usize = core::usize::MAX / 0xFF;
        const ONES_HIGH: usize = ONES << 7;
        let tmp = *self & (ONES * 127);
        (((ONES * (127 + b as usize) - tmp) & !*self & (tmp + (ONES * (127 - a as usize))))
            & ONES_HIGH)
            >> 7
    }

    #[inline(always)]
    fn bitand(&self, other: Self) -> Self {
        *self & other
    }

    #[inline(always)]
    fn add(&self, other: Self) -> Self {
        *self + other
    }

    #[inline(always)]
    fn sub(&self, other: Self) -> Self {
        *self - other
    }

    #[inline(always)]
    fn inc_nth_from_end_lex_byte(&self, n: usize) -> Self {
        if cfg!(target_endian = "little") {
            *self + (1 << ((Self::SIZE - 1 - n) * 8))
        } else {
            *self + (1 << (n * 8))
        }
    }

    #[inline(always)]
    fn dec_last_lex_byte(&self) -> Self {
        if cfg!(target_endian = "little") {
            *self - (1 << ((Self::SIZE - 1) * 8))
        } else {
            *self - 1
        }
    }

    #[inline(always)]
    fn sum_bytes(&self) -> usize {
        const ONES: usize = core::usize::MAX / 0xFF;
        self.wrapping_mul(ONES) >> ((Self::SIZE - 1) * 8)
    }
}

// Note: use only SSE2 and older instructions, since these are
// guaranteed on all x86_64 platforms.
#[cfg(target_arch = "x86_64")]
impl ByteChunk for x86_64::__m128i {
    const SIZE: usize = core::mem::size_of::<x86_64::__m128i>();
    const MAX_ACC: usize = 255;

    #[inline(always)]
    fn zero() -> Self {
        unsafe { x86_64::_mm_setzero_si128() }
    }

    #[inline(always)]
    fn splat(n: u8) -> Self {
        unsafe { x86_64::_mm_set1_epi8(n as i8) }
    }

    #[inline(always)]
    fn is_zero(&self) -> bool {
        let tmp = unsafe { core::mem::transmute::<Self, (u64, u64)>(*self) };
        tmp.0 == 0 && tmp.1 == 0
    }

    #[inline(always)]
    fn shift_back_lex(&self, n: usize) -> Self {
        match n {
            0 => *self,
            1 => unsafe { x86_64::_mm_srli_si128(*self, 1) },
            2 => unsafe { x86_64::_mm_srli_si128(*self, 2) },
            3 => unsafe { x86_64::_mm_srli_si128(*self, 3) },
            4 => unsafe { x86_64::_mm_srli_si128(*self, 4) },
            _ => unreachable!(),
        }
    }

    #[inline(always)]
    fn shr(&self, n: usize) -> Self {
        match n {
            0 => *self,
            1 => unsafe { x86_64::_mm_srli_epi64(*self, 1) },
            2 => unsafe { x86_64::_mm_srli_epi64(*self, 2) },
            3 => unsafe { x86_64::_mm_srli_epi64(*self, 3) },
            4 => unsafe { x86_64::_mm_srli_epi64(*self, 4) },
            _ => unreachable!(),
        }
    }

    #[inline(always)]
    fn cmp_eq_byte(&self, byte: u8) -> Self {
        let tmp = unsafe { x86_64::_mm_cmpeq_epi8(*self, Self::splat(byte)) };
        unsafe { x86_64::_mm_and_si128(tmp, Self::splat(1)) }
    }

    #[inline(always)]
    fn bytes_between_127(&self, a: u8, b: u8) -> Self {
        let tmp1 = unsafe { x86_64::_mm_cmpgt_epi8(*self, Self::splat(a)) };
        let tmp2 = unsafe { x86_64::_mm_cmplt_epi8(*self, Self::splat(b)) };
        let tmp3 = unsafe { x86_64::_mm_and_si128(tmp1, tmp2) };
        unsafe { x86_64::_mm_and_si128(tmp3, Self::splat(1)) }
    }

    #[inline(always)]
    fn bitand(&self, other: Self) -> Self {
        unsafe { x86_64::_mm_and_si128(*self, other) }
    }

    #[inline(always)]
    fn add(&self, other: Self) -> Self {
        unsafe { x86_64::_mm_add_epi8(*self, other) }
    }

    #[inline(always)]
    fn sub(&self, other: Self) -> Self {
        unsafe { x86_64::_mm_sub_epi8(*self, other) }
    }

    #[inline(always)]
    fn inc_nth_from_end_lex_byte(&self, n: usize) -> Self {
        let mut tmp = unsafe { core::mem::transmute::<Self, [u8; 16]>(*self) };
        tmp[15 - n] += 1;
        unsafe { core::mem::transmute::<[u8; 16], Self>(tmp) }
    }

    #[inline(always)]
    fn dec_last_lex_byte(&self) -> Self {
        let mut tmp = unsafe { core::mem::transmute::<Self, [u8; 16]>(*self) };
        tmp[15] -= 1;
        unsafe { core::mem::transmute::<[u8; 16], Self>(tmp) }
    }

    #[inline(always)]
    fn sum_bytes(&self) -> usize {
        let half_sum = unsafe { x86_64::_mm_sad_epu8(*self, x86_64::_mm_setzero_si128()) };
        let (low, high) = unsafe { core::mem::transmute::<Self, (u64, u64)>(half_sum) };
        (low + high) as usize
    }
}

//=============================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn usize_flag_bytes_01() {
        let v: usize = 0xE2_09_08_A6_E2_A6_E2_09;
        assert_eq!(0x00_00_00_00_00_00_00_00, v.cmp_eq_byte(0x07));
        assert_eq!(0x00_00_01_00_00_00_00_00, v.cmp_eq_byte(0x08));
        assert_eq!(0x00_01_00_00_00_00_00_01, v.cmp_eq_byte(0x09));
        assert_eq!(0x00_00_00_01_00_01_00_00, v.cmp_eq_byte(0xA6));
        assert_eq!(0x01_00_00_00_01_00_01_00, v.cmp_eq_byte(0xE2));
    }

    #[test]
    fn usize_bytes_between_127_01() {
        let v: usize = 0x7E_09_00_A6_FF_7F_08_07;
        assert_eq!(0x01_01_00_00_00_00_01_01, v.bytes_between_127(0x00, 0x7F));
        assert_eq!(0x00_01_00_00_00_00_01_00, v.bytes_between_127(0x07, 0x7E));
        assert_eq!(0x00_01_00_00_00_00_00_00, v.bytes_between_127(0x08, 0x7E));
    }

    #[cfg(all(feature = "simd", target_arch = "x86_64"))]
    #[test]
    fn sum_bytes_x86_64() {
        use core::arch::x86_64::__m128i as T;

        let ones = T::splat(1);
        let mut acc = T::zero();
        for _ in 0..T::MAX_ACC {
            acc = acc.add(ones);
        }

        assert_eq!(acc.sum_bytes(), T::SIZE * T::MAX_ACC);
    }
}
