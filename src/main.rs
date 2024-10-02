use std::time::{Duration, SystemTime};
use std::thread::sleep;

use std::fs;
use std::fs::{File, OpenOptions};
use std::io;
use std::io::prelude::*;
#[cfg(target_family = "unix")]
use std::os::unix;

use rand::{random, SeedableRng};

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

struct DisplayData {
    pixels: [[u8; 8]; 128],
}


impl DisplayData {
    fn print_display(&self) {
        for y in 0..128 {
            for x in 0..8 {
                for px in 0..8 {
                    if self.pixels[y][x] & (1 >> px) > 0 {
                        print!("*");
                    } else {
                        print!("x");
                    }
                }
            }
            println!("");
        }
    }

    fn draw(&self) {
        self.print_display();
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
        if self.pointer == 254 {
            return Err(self.pointer);
        }
        self.pointer += 1;
        self.data[self.pointer] = new_data;
     	return Ok(self.pointer);
    }
}

struct Sprite15 {
    data: [u8; 15],
    width: u8,
}

struct Sprite5 {
    data: [u8; 5],
    width: u8,
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
    kpad: KeyPad,
}

impl Cpu {
    fn inst_cls(&mut self) { //* CLear Screen CLS
        println!("CLS");
        let mut x = 0;
        let mut y = 0;

        while x < 8 {
            while y < 16 {
                self.display_mem.pixels[x][y] = 0;
                y += 1;
            }
            x += 1;
        }

    }
    fn inst_ret(&mut self) { //* RETurn RET
        println!("RET");
        self.pc.set_as(self.stack.pop());
    }
    fn inst_jp(&mut self, addr: u16) { //* JumP JP
        println!("JP");
        self.pc.set_as(addr);
    }
    fn inst_call(&mut self, addr: u16) { //* CALL subroutine CALL
        match self.stack.push(self.pc.get_cur()) { //? Check that we don't have to increment PC before doing this bc it seems sus
            Err(_cur_stack_ptr) => println!("FATAL: the stack is full"),
            Ok(_cur_stack_ptr) => self.pc.set_as(addr),
        }
    }
    fn inst_se_byte(&mut self, vreg_x: usize, byte: u8) { //* Conditional Skip Equal SE
        if self.gpreg.v[vreg_x] == byte {
            self.pc.incr_n(2);
        }
    }
    fn inst_se_reg(&mut self, vreg_x: usize, vreg_y: usize) { //* Conditional Skip Equal SE
        if self.gpreg.v[vreg_x] == self.gpreg.v[vreg_y]  {
            self.pc.incr_n(2);
        }
    }
    fn inst_sne_byte(&mut self, vreg_x: usize, byte: u8) { //* Conditional Skip Not Equal SNE
        if self.gpreg.v[vreg_x] != byte {
            self.pc.incr_n(2);
        }
    }
    fn inst_sne_reg(&mut self, vreg_x: usize, vreg_y: usize) { //* Conditional Skip Not Equal SNE
        if self.gpreg.v[vreg_x] != self.gpreg.v[vreg_y]  {
            self.pc.incr_n(2);
        }
    }
    fn inst_ld_byte(&mut self, vreg_x: usize, byte: u8) { //* LoaD byte LD
        self.gpreg.v[vreg_x] = byte;
    }
    fn inst_ld_reg(&mut self, vreg_x: usize, vreg_y: usize) { //* LoaD reg LD
        self.gpreg.v[vreg_x] = self.gpreg.v[vreg_y];
    }
    fn inst_add_byte(&mut self, vreg_x: usize, byte: u8) { //* ADD byte ADD
        self.gpreg.v[vreg_x] += byte;
    }
    fn inst_add_reg(&mut self, vreg_x: usize, vreg_y: usize) { //* ADD reg ADD
        self.gpreg.v[vreg_x] += self.gpreg.v[vreg_y];
    }
    fn inst_or(&mut self, vreg_x: usize, vreg_y: usize) { //*bitwise OR operator OR
        self.gpreg.v[vreg_x] |= self.gpreg.v[vreg_y];
    }
    fn inst_and(&mut self, vreg_x: usize, vreg_y: usize) {
        self.gpreg.v[vreg_x] &= self.gpreg.v[vreg_y];
    }
    fn inst_xor(&mut self, vreg_x: usize, vreg_y: usize) {
        self.gpreg.v[vreg_x] ^= self.gpreg.v[vreg_y];
    }
    fn inst_sub(&mut self, vreg_x: usize, vreg_y: usize) {
        if self.gpreg.v[vreg_x] > self.gpreg.v[vreg_y] {
            self.gpreg.v[0xF] = 1;
        } else {
            self.gpreg.v[0xF] = 1;
        }
        self.gpreg.v[vreg_x] -= self.gpreg.v[vreg_y];
    }
    fn inst_subn(&mut self, vreg_x: usize, vreg_y: usize) {
        if self.gpreg.v[vreg_y] > self.gpreg.v[vreg_x] {
            self.gpreg.v[0xF] = 1;
        } else {
            self.gpreg.v[0xF] = 1;
        }
        self.gpreg.v[vreg_x] = self.gpreg.v[vreg_y] - self.gpreg.v[vreg_x];
    }
    fn inst_shr(&mut self, vreg_x: usize) {
        self.gpreg.v[0xF] = self.gpreg.v[vreg_x] & 0b00000001;
        self.gpreg.v[vreg_x] /= 2;
    }
    fn inst_shl(&mut self, vreg_x: usize) {
        self.gpreg.v[0xF] = self.gpreg.v[vreg_x] & 0b10000000;
        self.gpreg.v[vreg_x] *= 2;
    }
    fn inst_ldi(&mut self, addr: u16) {
        self.gpreg.i = addr;
    }
    fn inst_jpv0(&mut self, addr: u16) { //* JumP to addr+V0 JP V0
        println!("JPV0");
        self.pc.set_as(addr + self.gpreg.v[0x0] as u16);
    }
    fn inst_rnd(&mut self, vreg_x: usize, byte: u8) {
        self.gpreg.v[vreg_x] = rand::random::<u8>() & byte;
    }
    fn inst_drw(&mut self, vreg_x: usize, vreg_y: usize, nibble: u8) {
        let mut it: usize = 0;
        while it < nibble as usize {
            let x = vreg_x;
            let mut y = vreg_y + it; //? Might have flipped this around idk yet, vertical mode in case hahaha
            if y >= 8 {
                y = 0;
            }
            self.display_mem.pixels[x][y] = self.mem.data[self.gpreg.i as usize + it];
            it += 1;
        }
    }
    fn inst_skp(&mut self, vreg_x: usize) { //* Skip Key Pressed
        if self.kpad.keys & (1 << self.gpreg.v[vreg_x]) > 0 {
            self.pc.incr_n(2);
        }
    }
    fn inst_sknp(&mut self, vreg_x: usize) { //* Skip Key Not Pressed
        if !self.kpad.keys & (1 << self.gpreg.v[vreg_x]) > 0 {
            self.pc.incr_n(2);
        }
    }
    fn inst_ldvdt(&mut self, vreg_x: usize) {
        self.gpreg.v[vreg_x] = self.spreg.d;
    }
    fn inst_ldk(&mut self, vreg_x: usize) {
        let init_keys = self.kpad.keys;
        while self.kpad.keys == init_keys {}
        let mut it = 0;
        while it < 16 {
            if (self.kpad.keys >> it) > 0 {
                self.gpreg.v[vreg_x] = it;
                break;
            }
        }
    }
    fn inst_lddt(&mut self, vreg_x: usize) { //* LoaD reg in Delay Timer LDDT
        self.spreg.d = self.gpreg.v[vreg_x];
    }
    fn inst_ldst(&mut self, vreg_x: usize) { //* LoaD reg in Sound Timer LDST
        self.spreg.s = self.gpreg.v[vreg_x];
    }
    fn inst_addi(&mut self, vreg_x: usize, vreg_y: usize) { //* ADD reg ADD
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
        let mut it = 0;
        while it <= vreg_x {
            self.mem.data[(self.gpreg.i as usize) + it] = self.gpreg.v[it];
        }
    }
    fn inst_ldfm(&mut self, vreg_x: usize) { //* LD[I] Load from mem*/
        let mut it = 0;
        while it <= vreg_x {
            self.gpreg.v[it] = self.mem.data[(self.gpreg.i as usize) + it];
        }
    }

    fn dispatch_operation(&mut self) {

        //? SUS: check endianness sigh
        let opcode_digits_hex: [u8; 4] = [
            self.opcode[0] & 0xF0 / 0x10,   //* Dxxx */
            self.opcode[0] & 0x0F,          //* xDxx */
            self.opcode[1] & 0xF0 / 0x10,   //* xxDx */
            self.opcode[1] & 0x0F,          //* xxxD */
        ];

        let opcode_as_u16: u16 = (self.opcode[0] as u16 * 0x100) + (self.opcode[1] as u16);

        let vreg_x = opcode_digits_hex[1] as usize;
        let vreg_y = opcode_digits_hex[2] as usize;
        let byte = self.opcode[1];
        let addr = opcode_as_u16 % 0x1000;
        let nibble = opcode_digits_hex[3];




        match opcode_digits_hex[0] {
            0x0 => {
                if opcode_as_u16 == 0x00E0 {
                    println!("CLS opcode");
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
                        println!("UNRECOGNISED OPCODE 0x{}", opcode_as_u16)
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
                        self.inst_addi(vreg_x, vreg_y);
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
                        self.inst_ldtm(vreg_x);
                    },
                    _ => {
                        println!("UNRECOGNISED OPCODE 0x{}", opcode_as_u16)
                    }
                }
            },
            _ => {
                println!("UNRECOGNISED OPCODE 0x{}", opcode_as_u16)
            }
        }
    }

    fn read_opcode(&mut self) {
        self.opcode[0] = self.mem.data[self.pc.get_cur() as usize];
        self.opcode[1] = self.mem.data[self.pc.get_cur() as usize + 1];
    }

    fn tick(&mut self) {
        self.read_opcode();
        self.dispatch_operation();
        self.pc.incr();
        self.display_mem.draw();
    }

    fn load_rom(&mut self, rom: Memory){
        self.mem = rom;
    }

    fn init_mem(&mut self) {
        for b in 0..81 {
            self.mem.data[(FONT_START_MEM_LOCATION + b) as usize] = DEFAULT_FONT[b as usize];
        }
    }



}

fn file_to_rom(file: fs::File) -> Memory {
    let mut buf: Vec<u8> = ;
    let mut mem: Memory = Memory {data: [0; 4096]};
    file.read_to_end(&mut buf);
    for i in 0..4096 {
        mem.data[i] = buf[i];
    }
    mem

}

fn main() {
    let mut mycpu = Cpu {
        mem: Memory {data: [0; 4096]},
        gpreg: GPRegisters {v: [0; 16], i: 0},
        spreg: SPRegisters {d:0, s: 0},
        stack: StackData {data: [0; 16], pointer: 0},
        pc: PC {c: 0},
        opcode: [0; 2],
        display_mem: DisplayData {pixels: [[0; 8]; 128]},
        kpad: KeyPad {keys: 0},
    };

    loop {
        let begin = SystemTime::now();
        mycpu.tick();
        let slp = Duration::from_millis(2);
        sleep(slp);
    }
}
