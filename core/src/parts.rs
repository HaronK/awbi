//The game is divided in 10 parts.
const GAME_NUM_PARTS: usize = 10;

pub const GAME_PART_FIRST: u16 = 0x3E80;
pub const GAME_PART1: u16 = 0x3E80;
pub const GAME_PART2: u16 = 0x3E81; //Introductino
pub const GAME_PART3: u16 = 0x3E82;
pub const GAME_PART4: u16 = 0x3E83; //Wake up in the suspended jail
pub const GAME_PART5: u16 = 0x3E84;
pub const GAME_PART6: u16 = 0x3E85; //BattleChar sequence
pub const GAME_PART7: u16 = 0x3E86;
pub const GAME_PART8: u16 = 0x3E87;
pub const GAME_PART9: u16 = 0x3E88;
pub const GAME_PART10: u16 = 0x3E89;
pub const GAME_PART_LAST: u16 = 0x3E89;

//For each part of the game, four resources are referenced.
pub const MEMLIST_PART_PALETTE: usize = 0;
pub const MEMLIST_PART_CODE: usize = 1;
pub const MEMLIST_PART_POLY_CINEMATIC: usize = 2;
pub const MEMLIST_PART_VIDEO2: usize = 3;

pub const MEMLIST_PART_NONE: usize = 0x00;

/*
    MEMLIST_PART_VIDEO1 and MEMLIST_PART_VIDEO2 are used to store polygons.

    It seems that:
    - MEMLIST_PART_VIDEO1 contains the cinematic polygons.
    - MEMLIST_PART_VIDEO2 contains the polygons for player and enemies animations.

    That would make sense since protection screen and cinematic game parts do not load MEMLIST_PART_VIDEO2.

*/
pub const MEM_LIST_PARTS: [[u8; 4]; GAME_NUM_PARTS] = [
    //MEMLIST_PART_PALETTE   MEMLIST_PART_CODE   MEMLIST_PART_VIDEO1   MEMLIST_PART_VIDEO2
    [0x14, 0x15, 0x16, 0x00], // protection screens
    [0x17, 0x18, 0x19, 0x00], // introduction cinematic
    [0x1A, 0x1B, 0x1C, 0x11],
    [0x1D, 0x1E, 0x1F, 0x11],
    [0x20, 0x21, 0x22, 0x11],
    [0x23, 0x24, 0x25, 0x00], // battlechar cinematic
    [0x26, 0x27, 0x28, 0x11],
    [0x29, 0x2A, 0x2B, 0x11],
    [0x7D, 0x7E, 0x7F, 0x00],
    [0x7D, 0x7E, 0x7F, 0x00], // password screen
];
