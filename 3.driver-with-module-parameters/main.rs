#![no_std]
#![feature(allocator_api)]

extern crate kernel;

use kernel::prelude::*;
use kernel::param::{self, Param};

module! {
    type: TestModule,
    name: b"parameters_test_module",
    author: b"d0u9",
    description: b"Module parameters test program",
    license: b"GPL",
}

struct TestModule {
    whom: Param<String>,
    howmany: Param<i32>,
}

impl KernelModule for TestModule {
    fn init(self) -> Result {
        pr_debug!("parameters test module is loaded\n");

        for i in 0..self.howmany {
            pr_info!("#{} Hello, {}\n", i, self.whom);
        }
        Ok(())
    }
}

impl TestModule {
    fn new() -> TestModule {
        TestModule {
            whom: Param::new("Mom".to_string(), param::Flags::READ_ONLY),
            howmany: Param::new(1, param::Flags::READ_ONLY),
        }
    }
}
