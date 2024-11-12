#![no_std]
#![feature(allocator_api, global_asm)]

extern crate kernel;

use kernel::prelude::*;
use kernel::chrdev;
use kernel::tty;
use kernel::file_operations;
use kernel::sync::SpinLock;
use kernel::device;

module! {
    type: TtyPrintkDriver,
    name: b"ttyprintk_driver",
    author: b"Samo Pogacnik (translated to Rust)",
    description: b"Kernel module for printk messages over a pseudo TTY",
    license: b"GPL",
}

const TPK_STR_SIZE: usize = 508;  // should be larger than the max expected line length
const TPK_MAX_ROOM: usize = 4096; // assuming 4KB for room
const TPK_PREFIX: &[u8] = b"\x01";  // KERN_SOH in C
static mut TPK_CURR: usize = 0;

struct TtyPrintkPort {
    port: tty::Port,
    spinlock: SpinLock<()>,
}

struct TtyPrintkDriver {
    ttyprintk_port: TtyPrintkPort,
    tty_driver: Option<tty::Driver>,
}

impl TtyPrintkDriver {
    fn tpk_flush(&mut self) {
        if unsafe { TPK_CURR > 0 } {
            let buffer = &mut [0u8; TPK_STR_SIZE + 4];
            buffer[unsafe { TPK_CURR }] = 0; // Null-terminate the string
            pr_info!("{:?}[U] {:?}", TPK_PREFIX, buffer);
            unsafe { TPK_CURR = 0 };
        }
    }

    fn tpk_printk(&mut self, buf: &[u8], count: usize) -> usize {
        let mut i = unsafe { TPK_CURR };

        if buf.is_empty() {
            self.tpk_flush();
            return i;
        }

        for (j, &byte) in buf.iter().enumerate().take(count) {
            if i >= TPK_STR_SIZE {
                buffer[i] = b'\\';
                self.tpk_flush();
            }

            match byte {
                b'\r' => {
                    self.tpk_flush();
                    if j + 1 < count && buf[j + 1] == b'\n' {
                        i += 1;
                    }
                }
                b'\n' => self.tpk_flush(),
                _ => {
                    buffer[i] = byte;
                    i += 1;
                }
            }
        }

        unsafe { TPK_CURR = i };
        count
    }
}

impl KernelModule for TtyPrintkDriver {
    fn init(self) -> Result {
        pr_info!("Initializing ttyprintk driver");

        // Initialize tty driver
        let mut tty_driver = tty::Driver::new(1);
        tty_driver.driver_name = b"ttyprintk_driver".to_vec();
        tty_driver.name = b"ttyprintk_driver".to_vec();
        tty_driver.major = 4; // Just an example, assign actual major number
        tty_driver.minor_start = 3;
        tty_driver.type_ = tty::TTY_DRIVER_TYPE_CONSOLE;
        tty_driver.init_termios = tty::default_termios();
        tty_driver.init_termios.c_oflag = tty::OPOST | tty::OCRNL | tty::ONOCR | tty::ONLRET;

        tty_driver.set_operations(&TTY_OPS);

        // Initialize tty port and spinlock
        let ttyprintk_port = TtyPrintkPort {
            port: tty::Port::default(),
            spinlock: SpinLock::new(()),
        };

        // Register the driver
        self.tty_driver = Some(tty_driver);
        pr_info!("TtyPrintkDriver initialized successfully");
        Ok(())
    }
}

impl Drop for TtyPrintkDriver {
    fn drop(&mut self) {
        pr_info!("Cleaning up ttyprintk driver");
        if let Some(driver) = &self.tty_driver {
            driver.destroy();
        }
    }
}

impl file_operations::FileOperations for TtyPrintkDriver {
    fn write(ctx: &kernel::file_operations::FileOpContext, buf: &[u8]) -> Result<usize> {
        pr_info!("Writing data: {:?}", buf);
        let mut tty_driver = ctx.driver_data::<TtyPrintkDriver>();
        let written = tty_driver.tpk_printk(buf, buf.len());
        Ok(written)
    }
}

impl file_operations::FileOpener for TtyPrintkDriver {
    fn open(ctx: &kernel::file_operations::FileOpContext) -> Result {
        let tty_driver = ctx.driver_data::<TtyPrintkDriver>();
        tty_driver.tpk_flush();
        Ok(())
    }
}

static TTY_OPS: tty::TTYOperations = tty::TTYOperations {
    open: tpk_open,
    close: tpk_close,
    write: tpk_write,
    write_room: tpk_write_room,
    ioctl: tpk_ioctl,
};

fn tpk_open(tty: &mut tty::TTY, filp: &kernel::file_operations::FileOpContext) -> Result {
    pr_info!("Opening ttyprintk device");
    tty.driver_data = Some(filp.clone());
    Ok(())
}

fn tpk_close(tty: &mut tty::TTY, filp: &kernel::file_operations::FileOpContext) -> Result {
    pr_info!("Closing ttyprintk device");
    Ok(())
}

fn tpk_write(tty: &mut tty::TTY, buf: &[u8]) -> Result<usize> {
    pr_info!("Writing data to ttyprintk device");
    let driver = tty.driver_data::<TtyPrintkDriver>();
    driver.tpk_printk(buf, buf.len());
    Ok(buf.len())
}

fn tpk_write_room(tty: &tty::TTY) -> usize {
    TPK_MAX_ROOM
}

fn tpk_ioctl(tty: &tty::TTY, cmd: u32, arg: u64) -> Result {
    pr_info!("Handling ioctl command: {}", cmd);
    match cmd {
        0x5401 => Err(kernel::Error::EINVAL),  // Example: Handle specific commands
        _ => Ok(()),
    }
}

static mut BUFFER: [u8; TPK_STR_SIZE + 4] = [0; TPK_STR_SIZE + 4];

#[no_mangle]
pub extern "C" fn ttyprintk_init() {
    pr_info!("Loading ttyprintk driver");
    // Additional initialization if needed
}

#[no_mangle]
pub extern "C" fn ttyprintk_exit() {
    pr_info!("Unloading ttyprintk driver");
    // Cleanup code
}

