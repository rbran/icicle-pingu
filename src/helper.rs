use anyhow::Result;
use icicle_mem::{perm, Mapping};

pub fn create_empty_memory(
    mem: &mut icicle_mem::Mmu,
    addr: Option<u64>,
    len: u64,
    perm: u8,
) -> Result<(u64, u64)> {
    let page_size = mem.page_size();
    let blocks = (len + (page_size - 1)) / page_size;
    let addr = match addr {
        Some(x) => x,
        None => {
            mem.find_free_memory(icicle_mem::AllocLayout::from_size_align(
                blocks * page_size,
                page_size,
            ))?
        }
    };
    let mapping = Mapping { perm, value: 0x00 };
    assert!((0..blocks).into_iter().all(|i| mem.map_memory_len(
        addr + (page_size * i),
        page_size,
        mapping,
    )));
    Ok((addr, blocks * page_size))
}

//pub fn create_null(mem: &mut icicle_mem::Mmu) -> Result<u64> {
//    create_empty_memory(mem, Some(0), 1024, perm::NONE).map(|(addr, _)| addr)
//}

//pub fn create_stack(mem: &mut icicle_mem::Mmu, len: u64) -> Result<u64> {
//    create_empty_memory(mem, None, len, perm::READ | perm::WRITE).map(|(addr, _)| addr)
//}
