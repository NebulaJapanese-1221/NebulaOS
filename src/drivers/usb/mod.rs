//! USB Host Controller Interface for NebulaOS.

pub mod uhci;

/// Trait representing a generic USB Host Controller.
pub trait UsbHostController {
    /// Initializes the controller hardware.
    fn init(&mut self);
    /// Resets the controller and the USB bus.
    fn reset(&mut self);
}