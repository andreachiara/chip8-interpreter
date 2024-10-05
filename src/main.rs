use std::ops::BitXorAssign;
use std::time::{Duration, SystemTime};
use std::thread::sleep;

use std::env;

use std::fs;
use std::io::{self, stdout};
#[cfg(target_family = "unix")]
use std::os::unix;

use std::io::Write;
use std::process;


use crossterm::event::{KeyEvent, KeyModifiers};
use crossterm::execute;
use rand::{random, SeedableRng};

use crossterm::{
    ExecutableCommand, QueueableCommand,
    terminal, cursor, style::{self, Stylize},
    Command, event::{poll, read, DisableMouseCapture, EnableMouseCapture,
        Event, Event::Key, KeyCode, KeyboardEnhancementFlags,
        PushKeyboardEnhancementFlags,
        PopKeyboardEnhancementFlags,
        KeyEventKind},
    queue,
    terminal::{disable_raw_mode, enable_raw_mode},
};

struct Memory {
    data: [u8; 4096],
}

struct GPRegisters {
    v: [u8; 16],
    i: u16,
}

struct SPRegisters {
    d: u8,
    s: u8,
}

struct KeyPad {
    keys: u16,
}


const SCREEN_HEIGHT: u8 = 32;
const SCREEN_WIDTH: u8 = 64;

struct DisplayData {
    pixels: [[bool; SCREEN_WIDTH as usize]; SCREEN_HEIGHT as usize],
}

impl DisplayData {
    fn print_display(&self) {
        for y in 0..SCREEN_HEIGHT {
            for x in 0..SCREEN_WIDTH {
                if self.pixels[y as usize][x as usize] {
                    print!("[]");
                } else {
                    print!("  ");
                }
                //print!(" ({:x},{:x}) ",x, y);
                //print!("{:8b}", self.pixels[y][x]);
            }
            println!("");
        }
        println!("==========================================================");

    }

    fn crossterm_draw(&self) {
        let mut stdout = io::stdout();

        let mut scr: String = "".to_string();

        for y in 0..SCREEN_HEIGHT {
            let mut line: String = "".to_string();
            for x in 0..SCREEN_WIDTH {

                if self.pixels[y as usize][x as usize] {
                    line.push_str("██");
                    //queue!(stdout, cursor::MoveTo(x,y), style::PrintStyledContent( "█".magenta()));
                } else {
                    line.push_str("  ");
                }
            }
            scr.push_str(&line);
            scr.push_str("\n");

        }
        stdout.execute(terminal::Clear(terminal::ClearType::All));
        queue!(stdout, cursor::MoveTo(0, 0), style::PrintStyledContent(scr.blue()));
        stdout.flush();
    }

    fn diff_draw(&self, other: &DisplayData) {
        let mut stdout = io::stdout();

        let mut scr: String = "".to_string();
        //stdout.execute(terminal::Clear(terminal::ClearType::All));

        for y in 0..SCREEN_HEIGHT {
            let mut line: String = "".to_string();
            for x in 0..SCREEN_WIDTH {

                if self.pixels[y as usize][x as usize] != other.pixels[y as usize][x as usize] {
                    if self.pixels[y as usize][x as usize] {
                        queue!(stdout, cursor::MoveTo(x as u16, y as u16), style::PrintStyledContent("██".blue()));
                        //queue!(stdout, cursor::MoveTo(x,y), style::PrintStyledContent( "█".magenta()));
                    } else {
                        queue!(stdout, cursor::MoveTo(x as u16, y as u16), style::PrintStyledContent("  ".blue()));
                    }
                }
            }
        }
        stdout.flush();
    }

    fn draw(&self) {
        //self.print_display();
        self.crossterm_draw();
    }
}

struct StackData {
    data: [u16; 16],
    pointer: usize,
}

impl StackData {
    fn pop(&mut self) -> u16 {
        let ret: u16 = self.data[self.pointer];
        if self.pointer > 0 {
            self.pointer -= 1;
        }
        ret
    }

    fn push(&mut self, new_data: u16) -> Result<usize, usize>{
        if self.pointer == 15 {
            return Err(self.pointer);
        }
        self.pointer += 1;
        self.data[self.pointer] = new_data;
     	return Ok(self.pointer);
    }
}

struct PC {
    c: u16,
}

const PC_INCR_UNIT: u16 = 2;

impl PC {
    fn incr(&mut self) {
        self.c += PC_INCR_UNIT;
    }
    fn incr_n(&mut self, n: u16) {
        self.c += n * PC_INCR_UNIT;
    }
    fn set_as(&mut self, n: u16) {
        self.c = n;
    }
    fn get_cur(&self) -> u16 {
        self.c
    }
}


const DEFAULT_FONT: [u8; 80] = [
    0xF0, 0x90, 0x90, 0x90, 0xF0, //0


    0x20, 0x60, 0x20, 0x20, 0x70, //1


    0xF0, 0x10, 0xF0, 0x80, 0xF0, //2


    0xF0, 0x10, 0xF0, 0x10, 0xF0, //3


    0x90, 0x90, 0xF0, 0x10, 0x10, //4


    0xF0, 0x80, 0xF0, 0x10, 0xF0, //5


    0xF0, 0x80, 0xF0, 0x90, 0xF0, //6


    0xF0, 0x10, 0x20, 0x40, 0x40, //7


    0xF0, 0x90, 0xF0, 0x90, 0xF0, //8


    0xF0, 0x90, 0xF0, 0x10, 0xF0, //9


    0xF0, 0x90, 0xF0, 0x90, 0x90, //A


    0xE0, 0x90, 0xE0, 0x90, 0xE0, //B


    0xF0, 0x80, 0x80, 0x80, 0xF0, //C


    0xE0, 0x90, 0x90, 0x90, 0xE0, //D


    0xF0, 0x80, 0xF0, 0x80, 0xF0, //E


    0xF0, 0x80, 0xF0, 0x80, 0x80, //F
];




const FONT_START_MEM_LOCATION: u16 = 0x50; //TODO: put the font somewhere it makes sense to
const FONT_WIDTH_BYTES: u8 = 5; //TODO: put the font somewhere it makes sense to

//fn name(par: type) -> ret

struct Cpu {
    mem: Memory,
    gpreg: GPRegisters,
    spreg: SPRegisters,
    stack: StackData,
    pc: PC,
    opcode: [u8; 2],
    display_mem: DisplayData,
    dp_old: DisplayData,
    kpad: KeyPad,
    kpad_old: KeyPad,
    no_pc_incr: bool,
}

impl Cpu {
    fn inst_cls(&mut self) { //* CLear Screen CLS
        //println!("CLS");

        for y in 0..SCREEN_HEIGHT {
            for x in 0..SCREEN_WIDTH {
                self.display_mem.pixels[y as usize][x as usize] = false;
            }
        }

    }
    fn inst_ret(&mut self) { //* RETurn RET
        //println!("RET");
        self.pc.set_as(self.stack.pop());
    }
    fn inst_jp(&mut self, addr: u16) { //* JumP JP
        //println!("JP");
        self.pc.set_as(addr);
        self.no_pc_incr = true;
    }
    fn inst_call(&mut self, addr: u16) { //* CALL subroutine CALL
        match self.stack.push(self.pc.get_cur()) { //? Check that we don't have to increment PC before doing this bc it seems sus
            Err(_cur_stack_ptr) => println!("FATAL: the stack is full"),
            Ok(_cur_stack_ptr) => self.pc.set_as(addr),
        }
        self.no_pc_incr = true;
    }
    fn inst_se_byte(&mut self, vreg_x: usize, byte: u8) { //* Conditional Skip Equal SE
        if self.gpreg.v[vreg_x] == byte {
            self.pc.incr_n(1);
        }
    }
    fn inst_se_reg(&mut self, vreg_x: usize, vreg_y: usize) { //* Conditional Skip Equal SE
        if self.gpreg.v[vreg_x] == self.gpreg.v[vreg_y]  {
            self.pc.incr_n(1);
        }
    }
    fn inst_sne_byte(&mut self, vreg_x: usize, byte: u8) { //* Conditional Skip Not Equal SNE
        if self.gpreg.v[vreg_x] != byte {
            self.pc.incr_n(1);
        }
    }
    fn inst_sne_reg(&mut self, vreg_x: usize, vreg_y: usize) { //* Conditional Skip Not Equal SNE
        if self.gpreg.v[vreg_x] != self.gpreg.v[vreg_y]  {
            self.pc.incr_n(1);
        }
    }
    fn inst_ld_byte(&mut self, vreg_x: usize, byte: u8) { //* LoaD byte LD
        self.gpreg.v[vreg_x] = byte;
    }
    fn inst_ld_reg(&mut self, vreg_x: usize, vreg_y: usize) { //* LoaD reg LD
        self.gpreg.v[vreg_x] = self.gpreg.v[vreg_y];
    }
    fn inst_add_byte(&mut self, vreg_x: usize, byte: u8) { //* ADD byte ADD
        self.gpreg.v[vreg_x] = self.gpreg.v[vreg_x].wrapping_add(byte);
    }
    fn inst_add_reg(&mut self, vreg_x: usize, vreg_y: usize) { //* ADD reg ADD
        let sumu16: u16 = (self.gpreg.v[vreg_x] as u16 + (self.gpreg.v[vreg_y] as u16));
        self.gpreg.v[vreg_x] = self.gpreg.v[vreg_x].wrapping_add(self.gpreg.v[vreg_y]);
        if sumu16 > 0xFF {
            self.gpreg.v[0xF] = 1;
        } else {
            self.gpreg.v[0xF] = 0;
        }
    }
    fn inst_or(&mut self, vreg_x: usize, vreg_y: usize) { //*bitwise OR operator OR
        self.gpreg.v[vreg_x] |= self.gpreg.v[vreg_y];
        self.gpreg.v[0xF] = 0; //quirk
    }
    fn inst_and(&mut self, vreg_x: usize, vreg_y: usize) {
        self.gpreg.v[vreg_x] &= self.gpreg.v[vreg_y];
        self.gpreg.v[0xF] = 0; //quirk
    }
    fn inst_xor(&mut self, vreg_x: usize, vreg_y: usize) {
        self.gpreg.v[vreg_x] ^= self.gpreg.v[vreg_y];
        self.gpreg.v[0xF] = 0; //quirk
    }
    fn inst_sub(&mut self, vreg_x: usize, vreg_y: usize) {
        let mut vf = 0;
        if self.gpreg.v[vreg_x] >= self.gpreg.v[vreg_y] {
            vf = 1;
        };
        self.gpreg.v[vreg_x] = self.gpreg.v[vreg_x].wrapping_sub(self.gpreg.v[vreg_y]);
        self.gpreg.v[0xF] = vf;
    }
    fn inst_subn(&mut self, vreg_x: usize, vreg_y: usize) {
        let mut vf = 0;
        if self.gpreg.v[vreg_y] >= self.gpreg.v[vreg_x] {
            vf = 1;
        }
        self.gpreg.v[vreg_x] = self.gpreg.v[vreg_y].wrapping_sub(self.gpreg.v[vreg_x]);
        self.gpreg.v[0xF] = vf;
    }
    fn inst_shr(&mut self, vreg_x: usize) {
        let vf = self.gpreg.v[vreg_x] & 0b00000001;
        self.gpreg.v[vreg_x] = self.gpreg.v[vreg_x] >> 1;
        self.gpreg.v[0xF] = vf;
    }
    fn inst_shl(&mut self, vreg_x: usize) {
        let vf = (self.gpreg.v[vreg_x] & 0b10000000) >> 7;
        self.gpreg.v[vreg_x] = self.gpreg.v[vreg_x] << 1;
        self.gpreg.v[0xF] = vf;

    }
    fn inst_ldi(&mut self, addr: u16) {
        self.gpreg.i = addr;
    }
    fn inst_jpv0(&mut self, addr: u16) { //* JumP to addr+V0 JP V0
        //println!("JPV0");
        self.pc.set_as(addr + self.gpreg.v[0x0] as u16);
    }
    fn inst_rnd(&mut self, vreg_x: usize, byte: u8) {
        self.gpreg.v[vreg_x] = rand::random::<u8>() & byte;
    }
    fn inst_drw(&mut self, vreg_x: usize, vreg_y: usize, nibble: u8) {
        let mut collision = false;
        //let mut it: usize = 0;
        for byte in 0..nibble {
            let y = self.gpreg.v[vreg_y].wrapping_add(byte) % SCREEN_HEIGHT;
            /*if y > SCREEN_HEIGHT-1 {
                break; //clipping is quirk
            }*/
            for px in 0..8 {
                let x = (self.gpreg.v[vreg_x].wrapping_add(px)) % SCREEN_WIDTH; //modulo only on x not px?
                let old_px = self.display_mem.pixels[y as usize][x as usize];
                self.display_mem.pixels[y as usize][x as usize] ^= ((self.mem.data[(self.gpreg.i + byte as u16) as usize] >> (7 - px)) & 0b1) > 0;
                //println!("{} {}", old_px, self.display_mem.pixels[y as usize][x as usize]);
                if old_px && !self.display_mem.pixels[y as usize][x as usize] {
                    collision = true;
                }
            }
        }

        if collision {
            self.gpreg.v[0xF] = 1;
        } else {
            self.gpreg.v[0xF] = 0;
        }
    }

    fn inst_skp(&mut self, vreg_x: usize) { //* Skip Key Not Pressed
        if ((self.kpad.keys >> self.gpreg.v[vreg_x]) & 0b1 > 0) {
            self.pc.incr_n(1);
        }
    }
    fn inst_sknp(&mut self, vreg_x: usize) { //* Skip Key Not Pressed
        if !((self.kpad.keys >> self.gpreg.v[vreg_x]) & 0b1 > 0) {
            self.pc.incr_n(1);
        }
    }
    fn inst_ldvdt(&mut self, vreg_x: usize) {
        self.gpreg.v[vreg_x] = self.spreg.d;
    }
    fn inst_ldk(&mut self, vreg_x: usize) {
        self.no_pc_incr = true;

        if self.kpad_old.keys == 0 {
            self.kpad_old.keys = self.kpad.keys;
        } else {
            if self.kpad.keys != self.kpad_old.keys {
                let mut it = 0;
                while it < 16 {
                    if (self.kpad.keys >> it) & 0b1 > 0 {
                        self.gpreg.v[vreg_x] = it;
                        break;
                    }
                    it += 1;
                }
                self.no_pc_incr = false;
                self.kpad_old.keys = 0;
            }
        }
    }
    fn inst_ldk_basic(&mut self, vreg_x: usize) {
        if self.kpad.keys == 0 {
            self.no_pc_incr = true;
        } else {
            for i in 0..16 {
                if (self.kpad.keys >> i) & 0b1 > 0 {
                    self.gpreg.v[vreg_x] = i;
                    break;
                }
            }
        }
    }
    fn inst_ldk_overcomplex(&mut self, vreg_x: usize) {
        while self.poll_keys().is_err() {}
        for i in 0..16 {
            if (self.kpad.keys >> i) & 0b1 > 0 {
                self.gpreg.v[vreg_x] = i;
                break;
            }
        }
        loop {
            while self.poll_keys().is_err() {}
            for i in 0..16 {
                if (self.kpad.keys >> self.gpreg.v[vreg_x]) & 0b1 == 0 {
                    return;
                }
            }
        }
    }
    fn inst_lddt(&mut self, vreg_x: usize) { //* LoaD reg in Delay Timer LDDT
        self.spreg.d = self.gpreg.v[vreg_x];
    }
    fn inst_ldst(&mut self, vreg_x: usize) { //* LoaD reg in Sound Timer LDST
        self.spreg.s = self.gpreg.v[vreg_x];
    }
    fn inst_addi(&mut self, vreg_x: usize) { //* ADD reg ADD
        self.gpreg.i += self.gpreg.v[vreg_x] as u16;
    }
    fn inst_ldf(&mut self, vreg_x: usize) { //* LoaD Font LDF (loads the memory location of the character for the digit stored in Vx) */
        self.gpreg.i = FONT_START_MEM_LOCATION + (vreg_x * FONT_WIDTH_BYTES as usize) as u16; // !assumes it is contiguous
    }
    fn inst_ldb(&mut self, vreg_x: usize) {
        self.mem.data[self.gpreg.i as usize] = self.gpreg.v[vreg_x] / 100;
        self.mem.data[self.gpreg.i as usize + 1] = (self.gpreg.v[vreg_x] % 100) / 10;
        self.mem.data[self.gpreg.i as usize + 2] = self.gpreg.v[vreg_x] % 10;
    }
    fn inst_ldtm(&mut self, vreg_x: usize) { //* LD[I] Load to mem*/
        for _i in 0..vreg_x+1 {
            self.mem.data[self.gpreg.i as usize] = self.gpreg.v[_i];//incr i is quirk
            self.gpreg.i += 1;
        }
    }
    fn inst_ldfm(&mut self, vreg_x: usize) { //* LD[I] Load from mem*/
        for _i in 0..vreg_x+1 {
            self.gpreg.v[_i] = self.mem.data[self.gpreg.i as usize]; //incr i is quirk
            self.gpreg.i += 1;
        }
    }

    fn dispatch_operation(&mut self) {

        //? SUS: check endianness sigh
        let opcode_digits_hex: [u8; 4] = [
            self.opcode[0] / 0x10,   //* Dxxx */
            self.opcode[0] & 0x0F,          //* xDxx */
            self.opcode[1] / 0x10,   //* xxDx */
            self.opcode[1] & 0x0F,          //* xxxD */
        ];

        //println!("opcode digits: {:x}|{:x}|{:x}|{:x}", opcode_digits_hex[0], opcode_digits_hex[1], opcode_digits_hex[2], opcode_digits_hex[3]);

        let opcode_as_u16: u16 = (self.opcode[0] as u16 * 0x100) + (self.opcode[1] as u16);

        let vreg_x = opcode_digits_hex[1] as usize;
        let vreg_y = opcode_digits_hex[2] as usize;
        let byte = self.opcode[1];
        let addr = opcode_as_u16 % 0x1000;
        let nibble = opcode_digits_hex[3];




        match opcode_digits_hex[0] {
            0x0 => {
                if opcode_as_u16 == 0x00E0 {
                    //println!("CLS opcode");
                    self.inst_cls();
                } else if opcode_as_u16 == 0x00EE {
                    self.inst_ret();
                }
            },
            0x1 => {
                self.inst_jp(addr);
            },
            0x2 => {
                self.inst_call(addr);
            },
            0x3 => {
                self.inst_se_byte(vreg_x, byte);
            },
            0x4 => {
                self.inst_sne_byte(vreg_x, byte);
            },
            0x5 => {
                if opcode_digits_hex[3] == 0 {
                    self.inst_se_reg(vreg_x, vreg_y);
                }
            },
            0x6 => {
                self.inst_ld_byte(vreg_x, byte);
            },
            0x7 => {
                self.inst_add_byte(vreg_x, byte);
            },
            0x8 => {
                match opcode_digits_hex[3] {
                    0x0 => {
                        self.inst_ld_reg(vreg_x, vreg_y);
                    },
                    0x1 => {
                        self.inst_or(vreg_x, vreg_y);
                    },
                    0x2 => {
                        self.inst_and(vreg_x, vreg_y);
                    },
                    0x3 => {
                        self.inst_xor(vreg_x, vreg_y);
                    },
                    0x4 => {
                        self.inst_add_reg(vreg_x, vreg_y);
                    },
                    0x5 => {
                        self.inst_sub(vreg_x, vreg_y);
                    },
                    0x6 => {
                        self.inst_shr(vreg_x);
                    },
                    0x7 => {
                        self.inst_subn(vreg_x, vreg_y);
                    },
                    0xE => {
                        self.inst_shl(vreg_x);
                    },
                    _ => {
                        //println!("UNRECOGNISED OPCODE 0x{:x}", opcode_as_u16)
                    }
                }
            },
            0x9 => {
                if opcode_digits_hex[3] == 0 {
                    self.inst_sne_reg(vreg_x, vreg_y);
                }
            },
            0xA => {
                self.inst_ldi(addr);
            },
            0xB => {
                self.inst_jpv0(addr);
            },
            0xC => {
                self.inst_rnd(vreg_x, byte);
            },
            0xD => {
                self.inst_drw(vreg_x, vreg_y, nibble);
            },
            0xE => {
                if self.opcode[1] == 0x9E {
                    self.inst_skp(vreg_x);
                } else if self.opcode[1] == 0xA1 {
                    self.inst_sknp(vreg_x);
                }
            },
            0xF => {
                match self.opcode[1] {
                    0x07 => {
                        self.inst_ldvdt(vreg_x);
                    },
                    0x0A => {
                        self.inst_ldk(vreg_x);
                    },
                    0x15 => {
                        self.inst_lddt(vreg_x);
                    },
                    0x18 => {
                        self.inst_ldst(vreg_x);
                    },
                    0x1E => {
                        self.inst_addi(vreg_x);
                    },
                    0x29 => {
                        self.inst_ldf(vreg_x);
                    },
                    0x33 => {
                        self.inst_ldb(vreg_x);
                    },
                    0x55 => {
                        self.inst_ldtm(vreg_x);
                    },
                    0x65 => {
                        self.inst_ldfm(vreg_x);
                    },
                    _ => {
                        //println!("UNRECOGNISED OPCODE 0x{:x}", opcode_as_u16)
                    }
                }
            },
            _ => {
                //println!("UNRECOGNISED OPCODE 0x{:x}", opcode_as_u16)
            }
        }
    }

    fn read_opcode(&mut self) {
        self.opcode[0] = self.mem.data[self.pc.get_cur() as usize];
        self.opcode[1] = self.mem.data[self.pc.get_cur() as usize + 1];
    }

    fn load_rom(&mut self, rom: Vec<u8>){
        for i in 0..rom.len() {
            self.mem.data[i + 0x200] = rom[i];
            if i % 2 == 1 {
                //println!("{:04x} || {:02x}{:02x} | {:08b} {:08b}", i-1+0x200, rom[i-1], rom[i], rom[i-1], rom[i]);
            }
        }
    }

    fn init_mem(&mut self) {
        for b in 0..80 {
            self.mem.data[(FONT_START_MEM_LOCATION + b) as usize] = DEFAULT_FONT[b as usize];
        }
    }

    fn decr_timers(&mut self) {
        if self.spreg.d > 0 {
            self.spreg.d -= 1;
        }
        if self.spreg.s > 0 {
            self.spreg.s -= 1;
        }
    }

    fn poll_keys(&mut self) -> Result<KeyPad, std::io::Error> {
        let _res_raw = enable_raw_mode();
        if poll(Duration::from_millis(1))? {

            match read()? {
                Key(evt) => {
                    //println!("code{} release? {}", evt.code, evt.kind == KeyEventKind::Release);
                    match evt.code {
                        KeyCode::Char('1') => {
                            if evt.kind == KeyEventKind::Press {
                                self.kpad.keys |= 1 << 0x1;
                            } else if evt.kind == KeyEventKind::Release {
                                self.kpad.keys &= !(1 << 0x1);
                            }
                        },
                        KeyCode::Char('2') => {
                            if evt.kind == KeyEventKind::Press {
                                self.kpad.keys |= 1 << 0x2;
                            } else if evt.kind == KeyEventKind::Release {
                                self.kpad.keys &= !(1 << 0x2);
                            }
                        },
                        KeyCode::Char('3') => {
                            if evt.kind == KeyEventKind::Press {
                                self.kpad.keys |= 1 << 0x3;
                            } else if evt.kind == KeyEventKind::Release {
                                self.kpad.keys &= !(1 << 0x3);
                            }
                        },
                        KeyCode::Char('4') => {
                            if evt.kind == KeyEventKind::Press {
                                self.kpad.keys |= 1 << 0xC;
                            } else if evt.kind == KeyEventKind::Release {
                                self.kpad.keys &= !(1 << 0xC);
                            }
                        },
                        KeyCode::Char('q') => {
                            if evt.kind == KeyEventKind::Press {
                                self.kpad.keys |= 1 << 0x4;
                            } else if evt.kind == KeyEventKind::Release {
                                self.kpad.keys &= !(1 << 0x4);
                            }
                        },
                        KeyCode::Char('w') => {
                            if evt.kind == KeyEventKind::Press {
                                self.kpad.keys |= 1 << 0x5;
                            } else if evt.kind == KeyEventKind::Release {
                                self.kpad.keys &= !(1 << 0x5);
                            }
                        },
                        KeyCode::Char('e') => {
                            if evt.kind == KeyEventKind::Press {
                                self.kpad.keys |= 1 << 0x6;
                            } else if evt.kind == KeyEventKind::Release {
                                self.kpad.keys &= !(1 << 0x6);
                            }
                        },
                        KeyCode::Char('r') => {
                            if evt.kind == KeyEventKind::Press {
                                self.kpad.keys |= 1 << 0xD;
                            } else if evt.kind == KeyEventKind::Release {
                                self.kpad.keys &= !(1 << 0xD);
                            }
                        },
                        KeyCode::Char('a') => {
                            if evt.kind == KeyEventKind::Press {
                                self.kpad.keys |= 1 << 0x7;
                            } else if evt.kind == KeyEventKind::Release {
                                self.kpad.keys &= !(1 << 0x7);
                            }
                        },
                        KeyCode::Char('s') => {
                            if evt.kind == KeyEventKind::Press {
                                self.kpad.keys |= 1 << 0x8;
                            } else if evt.kind == KeyEventKind::Release {
                                self.kpad.keys &= !(1 << 0x8);
                            }
                        },
                        KeyCode::Char('d') => {
                            if evt.kind == KeyEventKind::Press {
                                self.kpad.keys |= 1 << 0x9;
                            } else if evt.kind == KeyEventKind::Release {
                                self.kpad.keys &= !(1 << 0x9);
                            }
                        },
                        KeyCode::Char('f') => {
                            if evt.kind == KeyEventKind::Press {
                                self.kpad.keys |= 1 << 0xE;
                            } else if evt.kind == KeyEventKind::Release {
                                self.kpad.keys &= !(1 << 0xE);
                            }
                        },
                        KeyCode::Char('z') => {
                            if evt.kind == KeyEventKind::Press {
                                self.kpad.keys |= 1 << 0xA;
                            } else if evt.kind == KeyEventKind::Release {
                                self.kpad.keys &= !(1 << 0xA);
                            }
                        },
                        KeyCode::Char('x') => {
                            if evt.kind == KeyEventKind::Press {
                                self.kpad.keys |= 1 << 0x0;
                            } else if evt.kind == KeyEventKind::Release {
                                self.kpad.keys &= !(1 << 0x0);
                            }
                        },
                        KeyCode::Char('c') => {
                            if evt.kind == KeyEventKind::Press {
                                self.kpad.keys |= 1 << 0xB;
                            } else if evt.kind == KeyEventKind::Release {
                                self.kpad.keys &= !(1 << 0xB);
                            }
                        },
                        KeyCode::Char('v') => {
                            if evt.kind == KeyEventKind::Press {
                                self.kpad.keys |= 1 << 0xF;
                            } else if evt.kind == KeyEventKind::Release {
                                self.kpad.keys &= !(1 << 0xF);
                            }
                        },
                        KeyCode::Esc => {
                            while disable_raw_mode().is_err() {}
                            while execute!(stdout(), PopKeyboardEnhancementFlags).is_err() {}
                            process::exit(0x00);
                        },
                        _ => {
                            while disable_raw_mode().is_err() {}
                            return Err(std::io::Error::new(io::ErrorKind::InvalidInput, ""));
                        }
                    }
                    while disable_raw_mode().is_err() {}
                    return Ok(KeyPad {keys: self.kpad.keys});
                },
                _ => {}
            }

        }
        while disable_raw_mode().is_err() {}

            // It's guaranteed that read() won't block if `poll` returns `Ok(true)`
        return Err(std::io::Error::new(io::ErrorKind::InvalidInput, ""));

        //Err(std::io::Error::new(io::ErrorKind::InvalidData, "nothing to poll"))
    }

        fn tick(&mut self) {

            let _poll = self.poll_keys();

            self.read_opcode();
            //println!("opcode: 0x{:2x}{:2x} | pc: 0x{:x} ", self.opcode[1], self.opcode[0], self.pc.get_cur());
            self.dispatch_operation();
            if self.no_pc_incr {
                self.no_pc_incr = false;
            } else {
                self.pc.incr();
            }
            //self.display_mem.draw();
        }


}

fn file_to_rom(filename: &str) -> Vec<u8> {
    let res = fs::read(filename);

    match res {
        Ok(buf) => return buf,
        Err(why) => println!("FS ERROR!! {}", why),
    }
    return Vec::new();

}

fn main() {
    let mut mycpu = Cpu {
        mem: Memory {data: [0; 4096]},
        gpreg: GPRegisters {v: [0; 16], i: 0},
        spreg: SPRegisters {d:0, s: 0},
        stack: StackData {data: [0; 16], pointer: 0},
        pc: PC {c: 0x200},
        opcode: [0; 2],
        display_mem: DisplayData {pixels: [[false; SCREEN_WIDTH as usize]; SCREEN_HEIGHT as usize]},
        dp_old: DisplayData {pixels: [[false; SCREEN_WIDTH as usize]; SCREEN_HEIGHT as usize]},
        kpad: KeyPad {keys: 0},
        kpad_old: KeyPad {keys: 0},
        no_pc_incr: false,
    };
    let args: Vec<String> = env::args().collect();
    stdout().execute(terminal::Clear(terminal::ClearType::All));

    if args.len() < 2 {
        return;
    }

    mycpu.load_rom(file_to_rom(&args[1]));
    mycpu.init_mem();

    execute!(
        stdout(),
        PushKeyboardEnhancementFlags(
            KeyboardEnhancementFlags::REPORT_EVENT_TYPES
        )
    );

    let mut cycles = 0;

    let hz = 300;


    loop {
        cycles += 1;
        mycpu.tick();
        let slp = Duration::from_millis(1000/hz);
        sleep(slp);
        if cycles > (hz / 60) {
            mycpu.display_mem.draw();
            //mycpu.display_mem.diff_draw(&mycpu.dp_old);
            mycpu.dp_old.pixels = mycpu.display_mem.pixels;
            mycpu.decr_timers();
        }
    }
}
