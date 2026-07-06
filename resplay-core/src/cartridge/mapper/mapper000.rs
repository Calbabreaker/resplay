use crate::cartridge::Mapper;

/// INES designation for NROM boards
/// https://www.nesdev.org/wiki/NROM
/// No bank switching, maps entire range to the ROM and RAM
#[derive(Default, Debug, serde::Serialize, serde::Deserialize)]
pub struct Mapper000 {}

#[typetag::serde]
impl Mapper for Mapper000 {}

#[cfg(test)]
mod test {
    use crate::cartridge::mapper::test::create_test_catridge;

    #[test]
    fn prg_ram() {
        let mut cartridge = create_test_catridge(0, &[], &[]);
        cartridge.cpu_write(0x6000, 2);
        assert_eq!(cartridge.cpu_read(0x6000), Some(2));
    }

    #[test]
    fn test() {
        let mut cartridge = create_test_catridge(0, &[&[1, 2, 3]], &[&[69]]);
        assert_eq!(cartridge.cpu_read(0x8000), Some(1));
        assert_eq!(cartridge.cpu_read(0x8002), Some(3));
        assert_eq!(cartridge.cpu_read(0xc002), Some(0));

        assert_eq!(cartridge.ppu_read(0x0000), Some(69));
    }
}
