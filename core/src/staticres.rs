use crate::video::*;
use lazy_static::lazy_static;
use std::collections::HashMap;

#[rustfmt::skip::macros(lazy_static)]

lazy_static! {
    // pub static ref OPCODE_TABLE: Vec<FnMut()> = [];

    pub static ref FREQUENCE_TABLE: [u16; 40] = [
        0x0CFF, 0x0DC3, 0x0E91, 0x0F6F, 0x1056, 0x114E, 0x1259, 0x136C,
        0x149F, 0x15D9, 0x1726, 0x1888, 0x19FD, 0x1B86, 0x1D21, 0x1EDE,
        0x20AB, 0x229C, 0x24B3, 0x26D7, 0x293F, 0x2BB2, 0x2E4C, 0x3110,
        0x33FB, 0x370D, 0x3A43, 0x3DDF, 0x4157, 0x4538, 0x4998, 0x4DAE,
        0x5240, 0x5764, 0x5C9A, 0x61C8, 0x6793, 0x6E19, 0x7485, 0x7BBD,
    ];

    pub static ref FONT: [u8; 768] = [
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x10, 0x10, 0x10, 0x10, 0x10, 0x00, 0x10, 0x00,
        0x28, 0x28, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x24, 0x7E, 0x24, 0x24, 0x7E, 0x24, 0x00,
        0x08, 0x3E, 0x48, 0x3C, 0x12, 0x7C, 0x10, 0x00, 0x42, 0xA4, 0x48, 0x10, 0x24, 0x4A, 0x84, 0x00,
        0x60, 0x90, 0x90, 0x70, 0x8A, 0x84, 0x7A, 0x00, 0x08, 0x08, 0x10, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x06, 0x08, 0x10, 0x10, 0x10, 0x08, 0x06, 0x00, 0xC0, 0x20, 0x10, 0x10, 0x10, 0x20, 0xC0, 0x00,
        0x00, 0x44, 0x28, 0x10, 0x28, 0x44, 0x00, 0x00, 0x00, 0x10, 0x10, 0x7C, 0x10, 0x10, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x10, 0x10, 0x20, 0x00, 0x00, 0x00, 0x7C, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x10, 0x28, 0x10, 0x00, 0x00, 0x04, 0x08, 0x10, 0x20, 0x40, 0x00, 0x00,
        0x78, 0x84, 0x8C, 0x94, 0xA4, 0xC4, 0x78, 0x00, 0x10, 0x30, 0x50, 0x10, 0x10, 0x10, 0x7C, 0x00,
        0x78, 0x84, 0x04, 0x08, 0x30, 0x40, 0xFC, 0x00, 0x78, 0x84, 0x04, 0x38, 0x04, 0x84, 0x78, 0x00,
        0x08, 0x18, 0x28, 0x48, 0xFC, 0x08, 0x08, 0x00, 0xFC, 0x80, 0xF8, 0x04, 0x04, 0x84, 0x78, 0x00,
        0x38, 0x40, 0x80, 0xF8, 0x84, 0x84, 0x78, 0x00, 0xFC, 0x04, 0x04, 0x08, 0x10, 0x20, 0x40, 0x00,
        0x78, 0x84, 0x84, 0x78, 0x84, 0x84, 0x78, 0x00, 0x78, 0x84, 0x84, 0x7C, 0x04, 0x08, 0x70, 0x00,
        0x00, 0x18, 0x18, 0x00, 0x00, 0x18, 0x18, 0x00, 0x00, 0x00, 0x18, 0x18, 0x00, 0x10, 0x10, 0x60,
        0x04, 0x08, 0x10, 0x20, 0x10, 0x08, 0x04, 0x00, 0x00, 0x00, 0xFE, 0x00, 0x00, 0xFE, 0x00, 0x00,
        0x20, 0x10, 0x08, 0x04, 0x08, 0x10, 0x20, 0x00, 0x7C, 0x82, 0x02, 0x0C, 0x10, 0x00, 0x10, 0x00,
        0x30, 0x18, 0x0C, 0x0C, 0x0C, 0x18, 0x30, 0x00, 0x78, 0x84, 0x84, 0xFC, 0x84, 0x84, 0x84, 0x00,
        0xF8, 0x84, 0x84, 0xF8, 0x84, 0x84, 0xF8, 0x00, 0x78, 0x84, 0x80, 0x80, 0x80, 0x84, 0x78, 0x00,
        0xF8, 0x84, 0x84, 0x84, 0x84, 0x84, 0xF8, 0x00, 0x7C, 0x40, 0x40, 0x78, 0x40, 0x40, 0x7C, 0x00,
        0xFC, 0x80, 0x80, 0xF0, 0x80, 0x80, 0x80, 0x00, 0x7C, 0x80, 0x80, 0x8C, 0x84, 0x84, 0x7C, 0x00,
        0x84, 0x84, 0x84, 0xFC, 0x84, 0x84, 0x84, 0x00, 0x7C, 0x10, 0x10, 0x10, 0x10, 0x10, 0x7C, 0x00,
        0x04, 0x04, 0x04, 0x04, 0x84, 0x84, 0x78, 0x00, 0x8C, 0x90, 0xA0, 0xE0, 0x90, 0x88, 0x84, 0x00,
        0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0xFC, 0x00, 0x82, 0xC6, 0xAA, 0x92, 0x82, 0x82, 0x82, 0x00,
        0x84, 0xC4, 0xA4, 0x94, 0x8C, 0x84, 0x84, 0x00, 0x78, 0x84, 0x84, 0x84, 0x84, 0x84, 0x78, 0x00,
        0xF8, 0x84, 0x84, 0xF8, 0x80, 0x80, 0x80, 0x00, 0x78, 0x84, 0x84, 0x84, 0x84, 0x8C, 0x7C, 0x03,
        0xF8, 0x84, 0x84, 0xF8, 0x90, 0x88, 0x84, 0x00, 0x78, 0x84, 0x80, 0x78, 0x04, 0x84, 0x78, 0x00,
        0x7C, 0x10, 0x10, 0x10, 0x10, 0x10, 0x10, 0x00, 0x84, 0x84, 0x84, 0x84, 0x84, 0x84, 0x78, 0x00,
        0x84, 0x84, 0x84, 0x84, 0x84, 0x48, 0x30, 0x00, 0x82, 0x82, 0x82, 0x82, 0x92, 0xAA, 0xC6, 0x00,
        0x82, 0x44, 0x28, 0x10, 0x28, 0x44, 0x82, 0x00, 0x82, 0x44, 0x28, 0x10, 0x10, 0x10, 0x10, 0x00,
        0xFC, 0x04, 0x08, 0x10, 0x20, 0x40, 0xFC, 0x00, 0x3C, 0x30, 0x30, 0x30, 0x30, 0x30, 0x3C, 0x00,
        0x3C, 0x30, 0x30, 0x30, 0x30, 0x30, 0x3C, 0x00, 0x3C, 0x30, 0x30, 0x30, 0x30, 0x30, 0x3C, 0x00,
        0x3C, 0x30, 0x30, 0x30, 0x30, 0x30, 0x3C, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xFE,
        0x3C, 0x30, 0x30, 0x30, 0x30, 0x30, 0x3C, 0x00, 0x00, 0x00, 0x38, 0x04, 0x3C, 0x44, 0x3C, 0x00,
        0x40, 0x40, 0x78, 0x44, 0x44, 0x44, 0x78, 0x00, 0x00, 0x00, 0x3C, 0x40, 0x40, 0x40, 0x3C, 0x00,
        0x04, 0x04, 0x3C, 0x44, 0x44, 0x44, 0x3C, 0x00, 0x00, 0x00, 0x38, 0x44, 0x7C, 0x40, 0x3C, 0x00,
        0x38, 0x44, 0x40, 0x60, 0x40, 0x40, 0x40, 0x00, 0x00, 0x00, 0x3C, 0x44, 0x44, 0x3C, 0x04, 0x78,
        0x40, 0x40, 0x58, 0x64, 0x44, 0x44, 0x44, 0x00, 0x10, 0x00, 0x10, 0x10, 0x10, 0x10, 0x10, 0x00,
        0x02, 0x00, 0x02, 0x02, 0x02, 0x02, 0x42, 0x3C, 0x40, 0x40, 0x46, 0x48, 0x70, 0x48, 0x46, 0x00,
        0x10, 0x10, 0x10, 0x10, 0x10, 0x10, 0x10, 0x00, 0x00, 0x00, 0xEC, 0x92, 0x92, 0x92, 0x92, 0x00,
        0x00, 0x00, 0x78, 0x44, 0x44, 0x44, 0x44, 0x00, 0x00, 0x00, 0x38, 0x44, 0x44, 0x44, 0x38, 0x00,
        0x00, 0x00, 0x78, 0x44, 0x44, 0x78, 0x40, 0x40, 0x00, 0x00, 0x3C, 0x44, 0x44, 0x3C, 0x04, 0x04,
        0x00, 0x00, 0x4C, 0x70, 0x40, 0x40, 0x40, 0x00, 0x00, 0x00, 0x3C, 0x40, 0x38, 0x04, 0x78, 0x00,
        0x10, 0x10, 0x3C, 0x10, 0x10, 0x10, 0x0C, 0x00, 0x00, 0x00, 0x44, 0x44, 0x44, 0x44, 0x78, 0x00,
        0x00, 0x00, 0x44, 0x44, 0x44, 0x28, 0x10, 0x00, 0x00, 0x00, 0x82, 0x82, 0x92, 0xAA, 0xC6, 0x00,
        0x00, 0x00, 0x44, 0x28, 0x10, 0x28, 0x44, 0x00, 0x00, 0x00, 0x42, 0x22, 0x24, 0x18, 0x08, 0x30,
        0x00, 0x00, 0x7C, 0x08, 0x10, 0x20, 0x7C, 0x00, 0x60, 0x90, 0x20, 0x40, 0xF0, 0x00, 0x00, 0x00,
        0xFE, 0xFE, 0xFE, 0xFE, 0xFE, 0xFE, 0xFE, 0x00, 0x38, 0x44, 0xBA, 0xA2, 0xBA, 0x44, 0x38, 0x00,
        0x38, 0x44, 0x82, 0x82, 0x44, 0x28, 0xEE, 0x00, 0x55, 0xAA, 0x55, 0xAA, 0x55, 0xAA, 0x55, 0xAA,
    ];

    pub static ref VARIABLE_NAME_BY_INDEX: HashMap<u8, &'static str> = {
        let mut m = HashMap::new();
        m.insert(0x3C, "RANDOM_SEED");
        m.insert(0xDA, "LAST_KEYCHAR");
        m.insert(0xE5, "HERO_POS_UP_DOWN");
        m.insert(0xF4, "MUS_MARK");
        m.insert(0xF9, "SCROLL_Y");
        m.insert(0xFA, "HERO_ACTION");
        m.insert(0xFB, "HERO_POS_JUMP_DOWN");
        m.insert(0xFC, "HERO_POS_LEFT_RIGHT");
        m.insert(0xFD, "HERO_POS_MASK");
        m.insert(0xFE, "HERO_ACTION_POS_MASK");
        m.insert(0xFF, "PAUSE_SLICES");
        m
    };

    pub static ref STRINGS_TABLE_ENG: HashMap<u16, &'static str> = {
        let mut m = HashMap::new();
        m.insert(0x001, "P E A N U T  3000");
        m.insert(0x002, "Copyright  } 1990 Peanut Computer, Inc.\nAll rights reserved.\n\nCDOS Version 5.01");
        m.insert(0x003, "2");
        m.insert(0x004, "3");
        m.insert(0x005, ".");
        m.insert(0x006, "A");
        m.insert(0x007, "@");
        m.insert(0x008, "PEANUT 3000");
        m.insert(0x00A, "R");
        m.insert(0x00B, "U");
        m.insert(0x00C, "N");
        m.insert(0x00D, "P");
        m.insert(0x00E, "R");
        m.insert(0x00F, "O");
        m.insert(0x010, "J");
        m.insert(0x011, "E");
        m.insert(0x012, "C");
        m.insert(0x013, "T");
        m.insert(0x014, "Shield 9A.5f Ok");
        m.insert(0x015, "Flux % 5.0177 Ok");
        m.insert(0x016, "CDI Vector ok");
        m.insert(0x017, " %%%ddd ok");
        m.insert(0x018, "Race-Track ok");
        m.insert(0x019, "SYNCHROTRON");
        m.insert(0x01A, "E: 23%\ng: .005\n\nRK: 77.2L\n\nopt: g+\n\n Shield:\n1: OFF\n2: ON\n3: ON\n\nP~: 1\n");
        m.insert(0x01B, "ON");
        m.insert(0x01C, "-");
        m.insert(0x021, "|");
        m.insert(0x022, "--- Theoretical study ---");
        m.insert(0x023, " THE EXPERIMENT WILL BEGIN IN    SECONDS");
        m.insert(0x024, "  20");
        m.insert(0x025, "  19");
        m.insert(0x026, "  18");
        m.insert(0x027, "  4");
        m.insert(0x028, "  3");
        m.insert(0x029, "  2");
        m.insert(0x02A, "  1");
        m.insert(0x02B, "  0");
        m.insert(0x02C, "L E T ' S   G O");
        m.insert(0x031, "- Phase 0:\nINJECTION of particles\ninto synchrotron");
        m.insert(0x032, "- Phase 1:\nParticle ACCELERATION.");
        m.insert(0x033, "- Phase 2:\nEJECTION of particles\non the shield.");
        m.insert(0x034, "A  N  A  L  Y  S  I  S");
        m.insert(0x035, "- RESULT:\nProbability of creating:\n ANTIMATTER: 91.V %\n NEUTRINO 27:  0.04 %\n NEUTRINO 424: 18 %\n");
        m.insert(0x036, "   Practical verification Y/N ?");
        m.insert(0x037, "SURE ?");
        m.insert(0x038, "MODIFICATION OF PARAMETERS\nRELATING TO PARTICLE\nACCELERATOR (SYNCHROTRON).");
        m.insert(0x039, "       RUN EXPERIMENT ?");
        m.insert(0x03C, "t---t");
        m.insert(0x03D, "000 ~");
        m.insert(0x03E, ".20x14dd");
        m.insert(0x03F, "gj5r5r");
        m.insert(0x040, "tilgor 25%");
        m.insert(0x041, "12% 33% checked");
        m.insert(0x042, "D=4.2158005584");
        m.insert(0x043, "d=10.00001");
        m.insert(0x044, "+");
        m.insert(0x045, "*");
        m.insert(0x046, "% 304");
        m.insert(0x047, "gurgle 21");
        m.insert(0x048, "{{{{");
        m.insert(0x049, "Delphine Software");
        m.insert(0x04A, "By Eric Chahi");
        m.insert(0x04B, "  5");
        m.insert(0x04C, "  17");
        m.insert(0x12C, "0");
        m.insert(0x12D, "1");
        m.insert(0x12E, "2");
        m.insert(0x12F, "3");
        m.insert(0x130, "4");
        m.insert(0x131, "5");
        m.insert(0x132, "6");
        m.insert(0x133, "7");
        m.insert(0x134, "8");
        m.insert(0x135, "9");
        m.insert(0x136, "A");
        m.insert(0x137, "B");
        m.insert(0x138, "C");
        m.insert(0x139, "D");
        m.insert(0x13A, "E");
        m.insert(0x13B, "F");
        m.insert(0x13C, "        ACCESS CODE:");
        m.insert(0x13D, "PRESS BUTTON OR RETURN TO CONTINUE");
        m.insert(0x13E, "   ENTER ACCESS CODE");
        m.insert(0x13F, "   INVALID PASSWORD !");
        m.insert(0x140, "ANNULER");
        m.insert(0x141, "      INSERT DISK ?\n\n\n\n\n\n\n\n\nPRESS ANY KEY TO CONTINUE");
        m.insert(0x142, " SELECT SYMBOLS CORRESPONDING TO\n THE POSITION\n ON THE CODE WHEEL");
        m.insert(0x143, "    LOADING...");
        m.insert(0x144, "              ERROR");
        m.insert(0x15E, "LDKD");
        m.insert(0x15F, "HTDC");
        m.insert(0x160, "CLLD");
        m.insert(0x161, "FXLC");
        m.insert(0x162, "KRFK");
        m.insert(0x163, "XDDJ");
        m.insert(0x164, "LBKG");
        m.insert(0x165, "KLFB");
        m.insert(0x166, "TTCT");
        m.insert(0x167, "DDRX");
        m.insert(0x168, "TBHK");
        m.insert(0x169, "BRTD");
        m.insert(0x16A, "CKJL");
        m.insert(0x16B, "LFCK");
        m.insert(0x16C, "BFLX");
        m.insert(0x16D, "XJRT");
        m.insert(0x16E, "HRTB");
        m.insert(0x16F, "HBHK");
        m.insert(0x170, "JCGB");
        m.insert(0x171, "HHFL");
        m.insert(0x172, "TFBB");
        m.insert(0x173, "TXHF");
        m.insert(0x174, "JHJL");
        m.insert(0x181, " BY");
        m.insert(0x182, "ERIC CHAHI");
        m.insert(0x183, "         MUSIC AND SOUND EFFECTS");
        m.insert(0x184, " ");
        m.insert(0x185, "JEAN-FRANCOIS FREITAS");
        m.insert(0x186, "IBM PC VERSION");
        m.insert(0x187, "      BY");
        m.insert(0x188, " DANIEL MORAIS");
        m.insert(0x18B, "       THEN PRESS FIRE");
        m.insert(0x18C, " PUT THE PADDLE ON THE UPPER LEFT CORNER");
        m.insert(0x18D, "PUT THE PADDLE IN CENTRAL POSITION");
        m.insert(0x18E, "PUT THE PADDLE ON THE LOWER RIGHT CORNER");
        m.insert(0x258, "      Designed by ..... Eric Chahi");
        m.insert(0x259, "    Programmed by...... Eric Chahi");
        m.insert(0x25A, "      Artwork ......... Eric Chahi");
        m.insert(0x25B, "Music by ........ Jean-francois Freitas");
        m.insert(0x25C, "            Sound effects");
        m.insert(0x25D, "        Jean-Francois Freitas\n             Eric Chahi");
        m.insert(0x263, "              Thanks To");
        m.insert(0x264, "           Jesus Martinez\n\n          Daniel Morais\n\n        Frederic Savoir\n\n      Cecile Chahi\n\n    Philippe Delamarre\n\n  Philippe Ulrich\n\nSebastien Berthet\n\nPierre Gousseau");
        m.insert(0x265, "Now Go Out Of This World");
        m.insert(0x190, "Good evening professor.");
        m.insert(0x191, "I see you have driven here in your\nFerrari.");
        m.insert(0x192, "IDENTIFICATION");
        m.insert(0x193, "Monsieur est en parfaite sante.");
        m.insert(0x194, "Y\n");
        m.insert(0x193, "AU BOULOT !!!\n");
        m.insert(END_OF_STRING_DICTIONARY, "");
        m
    };

    pub static ref STRINGS_TABLE_DEMO: HashMap<u16, &'static str> = {
        let mut m = HashMap::new();
        m.insert(0x001, "P E A N U T  3000");
        m.insert(0x002, "Copyright  } 1990 Peanut Computer, Inc.\nAll rights reserved.\n\nCDOS Version 5.01");
        m.insert(0x003, "2");
        m.insert(0x004, "3");
        m.insert(0x005, ".");
        m.insert(0x006, "A");
        m.insert(0x007, "@");
        m.insert(0x008, "PEANUT 3000");
        m.insert(0x00A, "R");
        m.insert(0x00B, "U");
        m.insert(0x00C, "N");
        m.insert(0x00D, "P");
        m.insert(0x00E, "R");
        m.insert(0x00F, "O");
        m.insert(0x010, "J");
        m.insert(0x011, "E");
        m.insert(0x012, "C");
        m.insert(0x013, "T");
        m.insert(0x014, "Shield 9A.5f Ok");
        m.insert(0x015, "Flux % 5.0177 Ok");
        m.insert(0x016, "CDI Vector ok");
        m.insert(0x017, " %%%ddd ok");
        m.insert(0x018, "Race-Track ok");
        m.insert(0x019, "SYNCHROTRON");
        m.insert(0x01A, "E: 23%\ng: .005\n\nRK: 77.2L\n\nopt: g+\n\n Shield:\n1: OFF\n2: ON\n3: ON\n\nP~: 1\n");
        m.insert(0x01B, "ON");
        m.insert(0x01C, "-");
        m.insert(0x021, "|");
        m.insert(0x022, "--- Theoretical study ---");
        m.insert(0x023, " THE EXPERIMENT WILL BEGIN IN    SECONDS");
        m.insert(0x024, "  20");
        m.insert(0x025, "  19");
        m.insert(0x026, "  18");
        m.insert(0x027, "  4");
        m.insert(0x028, "  3");
        m.insert(0x029, "  2");
        m.insert(0x02A, "  1");
        m.insert(0x02B, "  0");
        m.insert(0x02C, "L E T ' S   G O");
        m.insert(0x031, "- Phase 0:\nINJECTION of particles\ninto synchrotron");
        m.insert(0x032, "- Phase 1:\nParticle ACCELERATION.");
        m.insert(0x033, "- Phase 2:\nEJECTION of particles\non the shield.");
        m.insert(0x034, "A  N  A  L  Y  S  I  S");
        m.insert(0x035, "- RESULT:\nProbability of creating:\n ANTIMATTER: 91.V %\n NEUTRINO 27:  0.04 %\n NEUTRINO 424: 18 %\n");
        m.insert(0x036, "   Practical verification Y/N ?");
        m.insert(0x037, "SURE ?");
        m.insert(0x038, "MODIFICATION OF PARAMETERS\nRELATING TO PARTICLE\nACCELERATOR (SYNCHROTRON).");
        m.insert(0x039, "       RUN EXPERIMENT ?");
        m.insert(0x03C, "t---t");
        m.insert(0x03D, "000 ~");
        m.insert(0x03E, ".20x14dd");
        m.insert(0x03F, "gj5r5r");
        m.insert(0x040, "tilgor 25%");
        m.insert(0x041, "12% 33% checked");
        m.insert(0x042, "D=4.2158005584");
        m.insert(0x043, "d=10.00001");
        m.insert(0x044, "+");
        m.insert(0x045, "*");
        m.insert(0x046, "% 304");
        m.insert(0x047, "gurgle 21");
        m.insert(0x048, "{{{{");
        m.insert(0x049, "Delphine Software");
        m.insert(0x04A, "By Eric Chahi");
        m.insert(0x04B, "  5");
        m.insert(0x04C, "  17");
        m.insert(0x12C, "0");
        m.insert(0x12D, "1");
        m.insert(0x12E, "2");
        m.insert(0x12F, "3");
        m.insert(0x130, "4");
        m.insert(0x131, "5");
        m.insert(0x132, "6");
        m.insert(0x133, "7");
        m.insert(0x134, "8");
        m.insert(0x135, "9");
        m.insert(0x136, "A");
        m.insert(0x137, "B");
        m.insert(0x138, "C");
        m.insert(0x139, "D");
        m.insert(0x13A, "E");
        m.insert(0x13B, "F");
        m.insert(0x13D, "PRESS BUTTON OR RETURN TO CONTINUE");
        m.insert(0x13E, "   ENTER ACCESS CODE");
        m.insert(0x13F, "   INVALID PASSWORD !");
        m.insert(0x140, "ANNULER");
        m.insert(0x141, "          INSERT DISK ?");
        m.insert(0x142, " SELECT SYMBOLS CORRESPONDING TO\n THE POSITION\n ON THE CODE WHEEL");
        m.insert(0x143, "    LOADING...");
        m.insert(0x144, "              ERROR");
        m.insert(0x181, " BY");
        m.insert(0x182, "ERIC CHAHI");
        m.insert(0x183, "         MUSIC AND SOUND EFFECTS");
        m.insert(0x184, " ");
        m.insert(0x185, "JEAN-FRANCOIS FREITAS");
        m.insert(0x186, "IBM PC VERSION");
        m.insert(0x187, "      BY");
        m.insert(0x188, " DANIEL MORAIS");
        m.insert(0x18B, "       THEN PRESS FIRE");
        m.insert(0x18C, " PUT THE PADDLE ON THE UPPER LEFT CORNER");
        m.insert(0x18D, "PUT THE PADDLE IN CENTRAL POSITION");
        m.insert(0x18E, "PUT THE PADDLE ON THE LOWER RIGHT CORNER");
        m.insert(0x1F4, "Over Two Years in the Making");
        m.insert(0x1F5, "   A New, State\nof the Art, Polygon\n  Graphics System");
        m.insert(0x1F6, "   Comes to the\nComputer With Full\n Screen Graphics");
        m.insert(0x1F7, "While conducting a nuclear fission\nexperiment at your local\nparticle accelerator ...");
        m.insert(0x1F8, "Nature decides to put a little\n    extra spin on the ball");
        m.insert(0x1F9, "And sends you ...");
        m.insert(0x1FA, "     Out of this World\nA Cinematic Action Adventure\n Coming soon to a computer\n      screen near you\n from Interplay Productions\n   coming soon to the IBM");
        m.insert(0x258, "      Designed by ..... Eric Chahi");
        m.insert(0x259, "    Programmed by...... Eric Chahi");
        m.insert(0x25A, "      Artwork ......... Eric Chahi");
        m.insert(0x25B, "Music by ........ Jean-francois Freitas");
        m.insert(0x25C, "            Sound effects");
        m.insert(0x25D, "        Jean-Francois Freitas\n             Eric Chahi");
        m.insert(0x263, "              Thanks To");
        m.insert(0x264, "           Jesus Martinez\n\n          Daniel Morais\n\n        Frederic Savoir\n\n      Cecile Chahi\n\n    Philippe Delamarre\n\n  Philippe Ulrich\n\nSebastien Berthet\n\nPierre Gousseau");
        m.insert(0x265, "Now Go Out Of This World");
        m.insert(0x190, "Good evening professor.");
        m.insert(0x191, "I see you have driven here in your\nFerrari.");
        m.insert(0x192, "IDENTIFICATION");
        m.insert(0x193, "Monsieur est en parfaite sante.");
        m.insert(0x194, "Y\n");
        m.insert(0x193, "AU BOULOT !!!\n");
        m
    };
}
