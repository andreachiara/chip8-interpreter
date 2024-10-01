use rand::random;

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
    keys: [bool; 16],
}

struct DisplayData {
    pixels: [[u8; 8]; 16],
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
}

struct Sprite5 {
    data: [u8; 5],
}

struct PC {
    c: u16,
}

const PC_INCR_UNIT: u16 = 1;

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

const DEFAULT_FONT_SPRITE_0: Sprite5 = { Sprite5 {
    data: [0xF0, 0x90, 0x90, 0x90, 0xF0],
}}; //TODO: add the other default font sprites

const FONT_START_MEM_LOCATION: u16 = 0; //TODO: put the font somewhere it makes sense to
const FONT_WIDTH_BYTES: u8 = 5; //TODO: put the font somewhere it makes sense to

//fn name(par: type) -> ret

struct Cpu {
    mem: Memory,
    gpreg: GPRegisters,
    spreg: SPRegisters,
    stack: StackData,
    pc: PC,
    opcode: u16,
    display_mem: DisplayData,
    kpad: KeyPad,
}

impl Cpu {
    fn inst_cls() { //* CLear Screen CLS
        println!("CLS");
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
    fn inst_se_byte(&mut self, vreg: usize, byte: u8) { //* Conditional Skip Equal SE
        if self.gpreg.v[vreg] == byte {
            self.pc.incr_n(2);
        }
    }
    fn inst_se_reg(&mut self, vreg_x: usize, vreg_y: usize) { //* Conditional Skip Equal SE
        if self.gpreg.v[vreg_x] == self.gpreg.v[vreg_y]  {
            self.pc.incr_n(2);
        }
    }
    fn inst_sne_byte(&mut self, vreg: usize, byte: u8) { //* Conditional Skip Not Equal SNE
        if self.gpreg.v[vreg] != byte {
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
    fn inst_or(&mut self, vreg_x: usize, byte: u8) { //*bitwise OR operator OR
        self.gpreg.v[vreg_x] |= byte;
    }
    fn inst_and(&mut self, vreg_x: usize, byte: u8) {
        self.gpreg.v[vreg_x] &= byte;
    }
    fn inst_xor(&mut self, vreg_x: usize, byte: u8) {
        self.gpreg.v[vreg_x] ^= byte;
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
        if self.kpad.keys[vreg_x] {
            self.pc.incr_n(2);
        }
    }
    fn inst_sknp(&mut self, vreg_x: usize) { //* Skip Key Not Pressed
        if !self.kpad.keys[vreg_x] {
            self.pc.incr_n(2);
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



}

fn main() {
    println!("Hello, world!");
}
