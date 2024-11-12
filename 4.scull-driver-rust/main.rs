#![no_std]
#![feature(allocator_api)]

extern crate kernel;

use kernel::prelude::*;
use kernel::file_operations;
use kernel::cdev::CDev;
use kernel::mutex::Mutex;
use kernel::slab::Slab;
use kernel::list::{ListHead, ListNode};

const SCULL_NR_DEVS: usize = 4; // Number of devices
const SCULL_BLOCK_SIZE: usize = 512; // Block size for each device

module! {
    type: ScullModule,
    name: b"scull_module",
    author: b"d0u9",
    description: b"A simple memory-based storage device",
    license: b"GPL",
}

#[derive(Debug)]
struct ScullBlock {
    data: [u8; SCULL_BLOCK_SIZE],
    offset: usize,
    block_list: ListNode,
}

#[derive(Debug)]
struct ScullDev {
    mutex: Mutex<()>,
    block_counter: usize,
    block_list: ListHead,
    cdev: CDev,
}

impl ScullDev {
    fn new() -> Self {
        ScullDev {
            mutex: Mutex::new(()),
            block_counter: 0,
            block_list: ListHead::new(),
            cdev: CDev::new(),
        }
    }

    fn trim(&mut self) {
        pr_debug!("scull_trim() is invoked\n");
        let mut current = self.block_list.head();
        while let Some(node) = current {
            let block = unsafe { node.as_mut::<ScullBlock>() };
            current = node.next();
            // Remove and clean up the block
            self.block_list.remove(node);
            unsafe { kernel::slab::dealloc(block) };
        }
        self.block_counter = 0;
    }
}

struct ScullModule {
    devs: [Option<ScullDev>; SCULL_NR_DEVS],
    major: u32,
    minor: u32,
}

impl KernelModule for ScullModule {
    fn init(self) -> Result {
        pr_info!("Scull module is loaded\n");
        
        let mut err = 0;
        let devno = kernel::chrdev::alloc_chrdev_region(None, self.minor, SCULL_NR_DEVS as u32, b"scull_module")?;
        self.major = kernel::major(devno);
        self.minor = kernel::minor(devno);
        
        for i in 0..SCULL_NR_DEVS {
            self.devs[i] = Some(ScullDev::new());
            let dev = self.devs[i].as_mut().unwrap();
            let devno = kernel::MKDEV(self.major, self.minor + i as u32);
            dev.cdev.add(devno, 1)?;

            // Additional initialization steps like creating device entries can go here.
        }

        Ok(())
    }
}

impl KernelModule for ScullModule {
    fn exit(self) -> Result {
        pr_info!("Scull module unloaded\n");

        for dev in self.devs.iter_mut() {
            if let Some(dev) = dev.take() {
                dev.cdev.del();
                dev.trim();
            }
        }
        kernel::chrdev::unregister_chrdev_region(MKDEV(self.major, self.minor), SCULL_NR_DEVS as u32)?;
        Ok(())
    }
}

impl file_operations::FileOpener for ScullModule {
    fn open(ctx: &kernel::file_operations::FileContext) -> Result {
        pr_debug!("open() is invoked\n");

        let dev = &mut ctx.private_data().as_mut::<ScullDev>();
        
        if ctx.flags().contains(file_operations::FileOpenFlag::WRITE_ONLY) {
            dev.mutex.lock_interruptible()?;
            dev.trim();
            dev.mutex.unlock();
        }
        
        Ok(())
    }
}

impl file_operations::FileCloser for ScullModule {
    fn release(ctx: &kernel::file_operations::FileContext) -> Result {
        pr_debug!("release() is invoked\n");
        Ok(())
    }
}

impl file_operations::FileReader for ScullModule {
    fn read(ctx: &kernel::file_operations::FileContext, buf: &mut [u8], count: usize, offset: usize) -> Result<usize> {
        pr_debug!("read() is invoked\n");

        let dev = &ctx.private_data().as_mut::<ScullDev>();
        let tblock = offset / SCULL_BLOCK_SIZE;
        let toffset = offset % SCULL_BLOCK_SIZE;

        dev.mutex.lock_interruptible()?;

        if tblock + 1 > dev.block_counter {
            return Ok(0); // End of file
        }

        let mut plist = &dev.block_list;
        for _ in 0..tblock + 1 {
            plist = plist.next();
        }

        let pblock = unsafe { plist.as_mut::<ScullBlock>() };
        if toffset >= pblock.offset {
            return Ok(0); // End of file
        }

        let read_count = count.min(pblock.offset - toffset);
        buf.copy_from_slice(&pblock.data[toffset..toffset + read_count]);

        pr_debug!("RD pos = {}, block = {}, offset = {}, read {} bytes\n", offset, tblock, toffset, read_count);
        
        dev.mutex.unlock();
        Ok(read_count)
    }
}

impl file_operations::FileWriter for ScullModule {
    fn write(ctx: &kernel::file_operations::FileContext, buf: &[u8], count: usize, offset: usize) -> Result<usize> {
        pr_debug!("write() is invoked\n");

        let dev = &ctx.private_data().as_mut::<ScullDev>();
        let tblock = offset / SCULL_BLOCK_SIZE;
        let toffset = offset % SCULL_BLOCK_SIZE;

        dev.mutex.lock_interruptible()?;

        let mut pblock: Option<&mut ScullBlock> = None;

        while tblock + 1 > dev.block_counter {
            let block = Slab::<ScullBlock>::new().alloc();
            pblock = Some(block);
            dev.block_list.add_tail(block);
            dev.block_counter += 1;
        }

        pblock = Some(dev.block_list.last().unwrap().as_mut::<ScullBlock>());
        let pblock = pblock.unwrap();

        let write_count = (SCULL_BLOCK_SIZE - toffset).min(count);
        pblock.data[toffset..toffset + write_count].copy_from_slice(&buf[0..write_count]);
        pblock.offset += write_count;

        pr_debug!("WR pos = {}, block = {}, offset = {}, write {} bytes\n", offset, tblock, toffset, write_count);

        dev.mutex.unlock();
        Ok(write_count)
    }
}
