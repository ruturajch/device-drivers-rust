#![no_std]
#![feature(allocator_api, global_asm)]

extern crate kernel;

use kernel::prelude::*;
use kernel::chrdev;
use kernel::tty;
use kernel::file_operations;
use kernel::sync::Mutex;

module! {
    type: MyTTYDriver,
    name: b"my_tty_driver",
    author: b"Ruturaj",
    description: b"A fake TTY device",
    license: b"GPL",
}

const LDD_TTY_MINOR_NR: usize = 256;
const LDD_TTY_MAX_ROOM: usize = 256;
const TTYAUX_MAJOR: u32 = 0;  // Placeholder for major number

struct MyTTYDriver {
    chrdev: Option<chrdev::Registration>,
    tty_ports: [tty::Port; LDD_TTY_MINOR_NR],
    tty_dev_table: [Option<Box<MyTTYDevice>>; LDD_TTY_MINOR_NR],
}

struct MyTTYDevice {
    index: usize,
    open_count: usize,
    mutex: Mutex<()>,
    tty: Option<tty::TTY>,
}

impl KernelModule for MyTTYDriver {
    fn init(self) -> Result {
        pr_info!("Initializing fake TTY driver");

        // Register the character device
        let chrdev = chrdev::Registration::new_pinned::<file_operations::FileOps>(
            b"my_tty_driver",
            TTYAUX_MAJOR,
        )?;

        // Initialize the TTY driver structure
        let mut tty_driver = tty::Driver::new(LDD_TTY_MINOR_NR);
        tty_driver.driver_name = b"my_tty_driver".to_vec();
        tty_driver.name = b"my_tty_driver".to_vec();
        tty_driver.major = TTYAUX_MAJOR;
        tty_driver.minor_start = 3;  // Start minor numbering at 3
        tty_driver.type_ = tty::TTY_DRIVER_TYPE_CONSOLE;

        // Set operations for the TTY driver
        tty_driver.set_operations(&TTY_OPS);

        // Initialize tty_ports and tty_dev_table
        let mut tty_ports = [tty::Port::default(); LDD_TTY_MINOR_NR];
        let mut tty_dev_table = [None; LDD_TTY_MINOR_NR];
        
        for i in 0..LDD_TTY_MINOR_NR {
            tty_ports[i].init();
            tty_dev_table[i] = None;
        }

        // Set the fields of MyTTYDriver
        self.chrdev = Some(chrdev);
        self.tty_ports = tty_ports;
        self.tty_dev_table = tty_dev_table;

        pr_info!("Fake TTY driver initialized");
        Ok(())
    }
}

impl MyTTYDevice {
    fn new(index: usize) -> Self {
        MyTTYDevice {
            index,
            open_count: 0,
            mutex: Mutex::new(()),
            tty: None,
        }
    }

    fn open(&mut self, tty: &mut tty::TTY) {
        pr_info!("Opening TTY device {}...", self.index);

        // Lock mutex to ensure safe access
        let _lock = self.mutex.lock();

        self.open_count += 1;
        self.tty = Some(tty.clone());

        pr_info!("Device {} opened", self.index);
    }

    fn close(&mut self) {
        pr_info!("Closing TTY device {}...", self.index);

        // Lock mutex to ensure safe access
        let _lock = self.mutex.lock();

        if self.open_count > 0 {
            self.open_count -= 1;
        }

        pr_info!("Device {} closed", self.index);
    }

    fn write(&self, buf: &[u8]) {
        pr_info!("Writing data to TTY device {}: {:?}", self.index, buf);
    }
}

impl file_operations::FileOpener for MyTTYDriver {
    fn open(ctx: &kernel::file_operations::FileOpContext) -> Result {
        let index = ctx.filp().minor() as usize;
        let device = &mut self.tty_dev_table[index];

        match device {
            Some(dev) => {
                dev.open(&mut ctx.tty);
                pr_info!("Device {} opened", index);
            }
            None => {
                let new_dev = MyTTYDevice::new(index);
                self.tty_dev_table[index] = Some(Box::new(new_dev));
            }
        }

        Ok(())
    }
}

impl file_operations::FileOperations for MyTTYDriver {
    fn write(ctx: &kernel::file_operations::FileOpContext, buf: &[u8]) -> Result<usize> {
        let index = ctx.filp().minor() as usize;
        if let Some(dev) = &self.tty_dev_table[index] {
            dev.write(buf);
        }
        Ok(buf.len())
    }

    fn release(ctx: &kernel::file_operations::FileOpContext) -> Result {
        let index = ctx.filp().minor() as usize;
        if let Some(dev) = &self.tty_dev_table[index] {
            dev.close();
        }
        pr_info!("Device {} released", index);
        Ok(())
    }
}

static TTY_OPS: tty::TTYOperations = tty::TTYOperations {
    open: ldd_tty_open,
    close: ldd_tty_close,
    write: ldd_tty_write,
    write_room: ldd_tty_write_room,
};

fn ldd_tty_open(tty: &mut tty::TTY, filp: &kernel::file_operations::FileOpContext) -> Result {
    pr_info!("Opening TTY device...");
    Ok(())
}

fn ldd_tty_close(tty: &mut tty::TTY, filp: &kernel::file_operations::FileOpContext) -> Result {
    pr_info!("Closing TTY device...");
    Ok(())
}

fn ldd_tty_write(tty: &mut tty::TTY, buf: &[u8]) -> Result<usize> {
    pr_info!("Writing to TTY device: {:?}", buf);
    Ok(buf.len())
}

fn ldd_tty_write_room(tty: &tty::TTY) -> usize {
    LDD_TTY_MAX_ROOM
}

impl Drop for MyTTYDriver {
    fn drop(&mut self) {
        pr_info!("Cleaning up fake TTY driver");
        for i in 0..LDD_TTY_MINOR_NR {
            if let Some(dev) = &self.tty_dev_table[i] {
                dev.close();
            }
        }
    }
}

impl Drop for MyTTYDevice {
    fn drop(&mut self) {
        pr_info!("Cleaning up TTY device {}", self.index);
    }
}
