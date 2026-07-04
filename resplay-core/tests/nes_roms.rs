use resplay_core::{Emulator, controller::Button};

// Test rom by kevtris https://www.qmtpro.com/~nes/misc/nestest.txt
#[test]
fn nestest() {
    let correct_logs = include_str!("nestest.log");

    let mut emu = Emulator::default();
    emu.load_nes_rom(&include_bytes!("nestest.nes")[..])
        .unwrap();
    emu.cpu.pc = 0xc000;

    let mut prev_disassem = "CPU RESET";
    for (i, correct_line) in correct_logs.lines().enumerate() {
        let mut line_split = correct_line.split("//");
        let correct_output = line_split.next().unwrap().trim();
        let emu_log = format!(
            "{:04X} A:{:02X} X:{:02X} Y:{:02X} P:{:02X} SP:{:02X} PPU:{: >3},{: >3} CYC:{}",
            emu.cpu.pc,
            emu.cpu.a,
            emu.cpu.x,
            emu.cpu.y,
            emu.cpu.flags.bits(),
            emu.cpu.sp,
            emu.cpu.bus.ppu.registers.scanline,
            emu.cpu.bus.ppu.registers.dot,
            emu.cpu.bus.cpu_cycles_total,
        );
        assert_eq!(
            emu_log, correct_output,
            "Incorrect output on line {i} executing: {prev_disassem}"
        );

        // Current line actually contains the state of the instruction executed last line
        prev_disassem = line_split.next().unwrap().trim();

        emu.cpu.execute_next().unwrap();
    }
}
