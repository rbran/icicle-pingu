use icicle_mem::{perm, Mmu};

pub trait StrlenTest {
    fn write_str(&self, mem: &mut Mmu, addr: u64);
    fn result(&self) -> u64;
    fn data_len(&self) -> u64;
}

pub struct StrlenTestStatic {
    data: &'static [u8],
    result: u64,
}

impl StrlenTest for StrlenTestStatic {
    fn write_str(&self, mem: &mut Mmu, addr: u64) {
        mem.write_bytes(addr, self.data, perm::NONE).unwrap();
    }
    fn result(&self) -> u64 {
        self.result
    }
    fn data_len(&self) -> u64 {
        self.data.len().try_into().unwrap()
    }
}

pub struct StrlenTestLong {
    data: u8,
    data_len: u32,
}

impl StrlenTest for StrlenTestLong {
    fn write_str(&self, mem: &mut Mmu, addr: u64) {
        // TODO improve that
        mem.write_bytes(
            addr,
            &vec![self.data; self.data_len.try_into().unwrap()],
            perm::NONE,
        )
        .unwrap();
    }
    fn result(&self) -> u64 {
        self.data_len.into()
    }
    fn data_len(&self) -> u64 {
        self.data_len.into()
    }
}

pub trait StrlenTests:
    Iterator<Item = Box<dyn StrlenTest>> + Clone + Sized
{
    fn max_len(&self) -> u64;
}

impl<I> StrlenTests for I
where
    I: Iterator<Item = Box<dyn StrlenTest>> + Clone + Sized,
{
    fn max_len(&self) -> u64 {
        self.clone().map(|test| test.data_len()).max().unwrap_or(0)
    }
}

pub fn all_tests() -> impl StrlenTests {
    const FUNNY_STRING: &str =
        "aç😂¢ŴƉǁǆǗǱȌȘȤȮȵȸḐṑẜẞẟểỻɖʭʺ   ̉ͶἢЉՃ٣דܣޓޓਦଖሶᓅ᠊ᡈ†‖⁷₧℧Ⅷ↷∧⍗␖Ⓜ┳▐▮♁🭂✺Ꮅࠕࡕ\x00";
    const TESTS_STATIC: [(&[u8], u32); 10] = [
        (b"test\x00", 4),
        (b"test\x00123", 4),
        (b"\x00", 0),
        (b"\x00test", 0),
        (b"test\xff\x00test", 5),
        (b"\x01\x02\x03\x04\r\n\x7F\x00", 7),
        (b"\xff\xfe\xfd\xfc\xa0\xa1\xa2\xa3\x00", 8),
        (b"\xff\x00\x21\x00", 1),
        (b"\x00\x20\x00\x01\x00", 0),
        (FUNNY_STRING.as_bytes(), FUNNY_STRING.len() as u32 - 1), //-1 for \x00
    ];
    const TESTS_LONG: [(u8, u32); 2] = [(0x01, 0x1234), (0xff, 0x4321)];
    let tests_static = TESTS_STATIC
        .into_iter()
        .map(|(data, result)| StrlenTestStatic {
            data,
            result: result.into(),
        })
        .map(|data| Box::new(data) as Box<dyn StrlenTest>);

    let tests_long = TESTS_LONG
        .into_iter()
        .map(|(data, data_len)| StrlenTestLong { data, data_len })
        .map(|data| Box::new(data) as Box<dyn StrlenTest>);

    tests_static.chain(tests_long)
}
