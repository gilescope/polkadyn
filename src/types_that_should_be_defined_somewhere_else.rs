use parity_scale_codec::Decode;
use parity_scale_codec::Error;
use parity_scale_codec::Input;

/// A phase of a block's execution.
#[derive(Clone, Decode, Debug, PartialEq, PartialOrd, Eq, Ord)]
pub enum Phase {
    /// Applying an extrinsic.
    ApplyExtrinsic(u32),
    /// Finalizing the block.
    Finalization,
    /// Initializing the block.
    Initialization,
}

/// Era period
pub type Period = u64;

/// Era phase
pub type EraPhase = u64;

/// An era to describe the longevity of a transaction.
#[derive(PartialEq, Eq, Clone, Copy, Debug)]
// #[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub enum Era {
    /// The transaction is valid forever. The genesis hash must be present in the signed content.
    Immortal,

    /// Period and phase are encoded:
    /// - The period of validity from the block hash found in the signing material.
    /// - The phase in the period that this transaction's lifetime begins (and, importantly,
    /// implies which block hash is included in the signature material). If the `period` is
    /// greater than 1 << 12, then it will be a factor of the times greater than 1<<12 that
    /// `period` is.
    ///
    /// When used on `FRAME`-based runtimes, `period` cannot exceed `BlockHashCount` parameter
    /// of `system` module.
    Mortal(Period, EraPhase),
}

impl Decode for Era {
    fn decode<I: Input>(input: &mut I) -> Result<Self, Error> {
        let first = input.read_byte()?;
        if first == 0 {
            Ok(Self::Immortal)
        } else {
            let encoded = first as u64 + ((input.read_byte()? as u64) << 8);
            let period = 2 << (encoded % (1 << 4));
            let quantize_factor = (period >> 12).max(1);
            let phase = (encoded >> 4) * quantize_factor;
            if period >= 4 && phase < period {
                Ok(Self::Mortal(period, phase))
            } else {
                if period < 4 {
                    Err("period too low".into())
                } else {
                    Err("phase >= period".into())
                }
            }
        }
    }
}
