use anyhow::{bail, Result};
use icicle_mem::perm;

use crate::helper;

pub enum Param<'a, 'b> {
    /// this usize is the param
    Usize(u64),
    /// a 32 bits float
    F32(f32),
    /// a 64 bits float (duble)
    F64(f64),
    /// put the data to the heap and add a pointer as a param
    HeapData(&'a [u8]),
    /// the Fn will put the data to the heap and return an point as a param
    HeapFn(Box<dyn FnMut(&mut IcicleHelper) -> Result<u64> + 'b>),
}

pub enum Return {
    /// Is a simple usize value
    Usize(u64),
    /// a 32 bits float
    F32(f32),
    /// a 64 bits float (duble)
    F64(f64),
    /// value is an addr, read the CString it points to
    CString(Vec<u8>),
}

pub trait Vm {
    fn helper(&self) -> &IcicleHelper;
    fn helper_mut(&mut self) -> &mut IcicleHelper;
    fn lookup_symbol(&mut self, function_sym: &'static str) -> u64 {
        self.helper_mut()
            .icicle
            .env
            .lookup_symbol(function_sym)
            .unwrap()
    }
    fn call(
        &mut self,
        function_addr: u64,
        return_addr: u64,
        params: &mut [Param],
        results: &mut [Return],
    ) -> Result<()>;
}

pub struct IcicleHelper {
    pub icicle: icicle_vm::Vm,
    pub stack_addr: u64,
    pub stack_size: u64,
    pub stack_max: u64,
    pub heap_addr: u64,
    pub heap_used: u64,
    pub heap_size: u64,
    pub heap_max: u64,
}

impl IcicleHelper {
    pub fn new(
        icicle: icicle_vm::Vm,
        stack_addr: u64,
        stack_max: u64,
        heap_addr: u64,
        heap_max: u64,
    ) -> Self {
        Self {
            icicle,
            stack_addr,
            stack_size: 0,
            stack_max,
            heap_addr,
            heap_used: 0,
            heap_size: 0,
            heap_max,
        }
    }

    /// add this data to the stack
    pub fn set_stack_len(&mut self, len: u64) -> Result<()> {
        if len > self.stack_max {
            bail!("Stack is too big")
        }
        if len > self.stack_size {
            let grow = len - self.stack_size;
            let (_addr, size) = helper::create_empty_memory(
                &mut self.icicle.cpu.mem,
                Some(self.stack_addr + self.stack_size),
                grow,
                perm::READ | perm::WRITE,
            )?;
            self.stack_size += size;
        }
        Ok(())
    }

    //TODO allign this
    pub fn malloc(&mut self, size: u64) -> Result<u64> {
        let heap_available = self.heap_size - self.heap_used;
        if heap_available < size {
            let grow = size - heap_available;
            if self.heap_size + grow > self.heap_max {
                bail!("heap is too big")
            }
            let (_addr, size) = helper::create_empty_memory(
                &mut self.icicle.cpu.mem,
                Some(self.heap_addr + self.heap_size),
                grow,
                perm::READ | perm::WRITE,
            )?;
            self.heap_size += size;
        }
        let addr = self.heap_addr + self.heap_used;
        self.heap_used += size;
        debug_assert!(self.heap_used <= self.heap_size);
        Ok(addr)
    }

    // free all the heap
    pub fn free_all(&mut self) {
        self.heap_used = 0;
    }
}
