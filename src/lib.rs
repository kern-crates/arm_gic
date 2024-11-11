//! ARM Generic Interrupt Controller (GIC) register definitions and basic
//! operations.
//! Driver for the Arm Generic Interrupt Controller version 2 or 3 or 4, on aarch64.
//!
//! This top level module contains functions that are not specific to any particular interrupt
//! controller, as support for other GIC versions may be added in future.
//!
//! Note:
//!  - Interrupt grouping(secure state) is not supported
//!  - Interrupt proiority(preempt) is not supported
//!
//! Please contact the developer if you need this function

#![no_std]
#![feature(const_ptr_as_ref)]
#![feature(const_option)]
#![feature(const_nonnull_new)]

use core::fmt;
use core::fmt::{Debug, Formatter};

mod gic_v2;
mod gic_v3;
mod sysregs;

pub(crate) mod registers;

pub use crate::gic_v2::GicV2;
pub use crate::gic_v3::GicV3;

/// An interrupt ID.
#[derive(Copy, Clone, Eq, Ord, PartialOrd, PartialEq)]
pub struct IntId(usize);

impl IntId {
    /// Maximum number of interrupts supported by the GIC.
    pub const GIC_MAX_IRQ: usize = 1020;

    /// The ID of the first Software Generated Interrupt.
    const SGI_START: usize = 0;

    /// The ID of the first Private Peripheral Interrupt.
    const PPI_START: usize = 16;

    /// The ID of the first Shared Peripheral Interrupt.
    const SPI_START: usize = 32;

    /// The first special interrupt ID.
    const SPECIAL_START: usize = 1020;

    /// Returns the interrupt ID for the given Software Generated Interrupt.
    pub const fn sgi(sgi: usize) -> Self {
        assert!(sgi < Self::PPI_START);
        Self(Self::SGI_START + sgi)
    }

    /// Returns the interrupt ID for the given Private Peripheral Interrupt.
    pub const fn ppi(ppi: usize) -> Self {
        assert!(ppi < Self::SPI_START - Self::PPI_START);
        Self(Self::PPI_START + ppi)
    }

    /// Returns the interrupt ID for the given Shared Peripheral Interrupt.
    pub const fn spi(spi: usize) -> Self {
        assert!(spi < Self::SPECIAL_START);
        Self(Self::SPI_START + spi)
    }

    /// Returns whether this interrupt ID is for a Software Generated Interrupt.
    #[allow(dead_code)]
    fn is_sgi(self) -> bool {
        self.0 < Self::PPI_START
    }

    /// Returns whether this interrupt ID is private to a core, i.e. it is an SGI or PPI.
    #[allow(dead_code)]
    fn is_private(self) -> bool {
        self.0 < Self::SPI_START
    }
}

/// Different types of interrupt that the GIC handles.
pub enum InterruptType {
    /// Software-generated interrupt.
    ///
    /// SGIs are typically used for inter-processor communication and are
    /// generated by a write to an SGI register in the GIC.
    SGI,
    /// Private Peripheral Interrupt.
    ///
    /// Peripheral interrupts that are private to one core.
    PPI,
    /// Shared Peripheral Interrupt.
    ///
    /// Peripheral interrupts that can delivered to any connected core.
    SPI,
}

/// Translate an interrupt of a given type to a GIC INTID.
pub const fn translate_irq(id: usize, int_type: InterruptType) -> Option<usize> {
    match int_type {
        InterruptType::SGI => {
            if id < IntId::PPI_START {
                Some(id)
            } else {
                None
            }
        }
        InterruptType::PPI => {
            if id < IntId::SPI_START - IntId::PPI_START {
                Some(id + IntId::PPI_START)
            } else {
                None
            }
        }
        InterruptType::SPI => {
            if id < IntId::SPECIAL_START {
                Some(id + IntId::SPI_START)
            } else {
                None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_translate_irq() {
        assert_eq!(translate_irq(0, InterruptType::SGI), Some(0));
        assert_eq!(translate_irq(0, InterruptType::PPI), Some(16));
        assert_eq!(translate_irq(0, InterruptType::SPI), Some(32));
        assert_eq!(translate_irq(16, InterruptType::SGI), None);
        assert_eq!(translate_irq(16, InterruptType::PPI), None);
        assert_eq!(translate_irq(16, InterruptType::SPI), Some(48));
        assert_eq!(translate_irq(32, InterruptType::SGI), None);
        assert_eq!(translate_irq(32, InterruptType::PPI), None);
        assert_eq!(translate_irq(32, InterruptType::SPI), Some(64));
    }
}

impl Debug for IntId {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        if self.0 < Self::PPI_START {
            write!(f, "SGI {}", self.0 - Self::SGI_START)
        } else if self.0 < Self::SPI_START {
            write!(f, "PPI {}", self.0 - Self::PPI_START)
        } else if self.0 < Self::SPECIAL_START {
            write!(f, "SPI {}", self.0 - Self::SPI_START)
        } else {
            write!(f, "Special IntId {}", self.0)
        }
    }
}

impl From<IntId> for u32 {
    fn from(intid: IntId) -> Self {
        intid.0 as u32
    }
}

impl From<IntId> for usize {
    fn from(intid: IntId) -> Self {
        intid.0
    }
}

impl From<usize> for IntId {
    fn from(id: usize) -> Self {
        Self(id)
    }
}

/// Interrupt trigger mode.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum TriggerMode {
    /// Edge-triggered.
    ///
    /// This is an interrupt that is asserted on detection of a rising edge of
    /// an interrupt signal and then, regardless of the state of the signal,
    /// remains asserted until it is cleared by the conditions defined by this
    /// specification.
    Edge = 0,
    /// Level-sensitive.
    ///
    /// This is an interrupt that is asserted whenever the interrupt signal
    /// level is active, and deasserted whenever the level is not active.
    Level = 1,
}

/// [`GenericArmGic`].
/// It is used to implement the interface abstraction that the interrupt chip
/// driver should provide to the outside world.
/// I hope that the versatility of the driver interface should support more chip architectures.
pub trait GenericArmGic: Debug + Clone + Copy + Sync + Send + Sized {
    /// Initialises the GIC.
    fn init_primary(&mut self);

    /// Initialises the GIC for the current CPU core.
    fn per_cpu_init(&mut self);

    /// Configures the trigger type for the interrupt with the given ID.
    fn set_trigger(&mut self, intid: IntId, trigger: TriggerMode);

    /// Enables the interrupt with the given ID.pub fn enable_interrupt(&mut self, intid: IntId);
    fn enable_interrupt(&mut self, intid: IntId);

    /// Disable the interrupt with the given ID.
    fn disable_interrupt(&mut self, intid: IntId);

    /// Gets the ID of the highest priority signalled interrupt, and acknowledges it.
    ///
    /// Returns `None` if there is no pending interrupt of sufficient priority.
    fn get_and_acknowledge_interrupt(&self) -> Option<IntId>;

    /// Informs the interrupt controller that the CPU has completed processing the given interrupt.
    /// This drops the interrupt priority and deactivates the interrupt.
    fn end_interrupt(&self, intid: IntId);
}
