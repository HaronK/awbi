use crate::system::*;
use crate::{file::File, slice_reader::SliceReader};
use crate::{resource::*, util::w_sub};
use crate::{serializer::*, util::w_add_u32};
use crate::{staticres::*, util::w_mul_i16};
use anyhow::Result;
use std::cmp::Ordering;

struct StrEntry {
    id: u16,
    val: String,
}

#[derive(Clone, Copy, Default)]
pub struct Point {
    x: i16,
    y: i16,
}

impl Point {
    pub fn new(x: i16, y: i16) -> Self {
        Self { x, y }
    }
}

const MAX_POINTS: usize = 50;

struct Polygon {
    bbw: u16,
    bbh: u16,
    num_points: u8,
    points: [Point; MAX_POINTS],
}

impl Polygon {
    fn new() -> Self {
        Self {
            bbw: 0,
            bbh: 0,
            num_points: 0,
            points: [Default::default(); MAX_POINTS],
        }
    }

    fn read_vertices(&mut self, buf: &mut SliceReader, zoom: u16) {
        self.bbw = buf.read_u8() as u16 * zoom / 64;
        self.bbh = buf.read_u8() as u16 * zoom / 64;
        self.num_points = buf.read_u8();
        assert!((self.num_points & 1) == 0 && (self.num_points as usize) < MAX_POINTS);

        //Read all points, directly from bytecode segment
        for pt in &mut self.points[0..self.num_points as usize] {
            pt.x = (buf.read_u8() as u16 * zoom / 64) as i16;
            pt.y = (buf.read_u8() as u16 * zoom / 64) as i16;
        }
    }
}

// This is used to detect the end of  _stringsTableEng and _stringsTableDemo
pub const END_OF_STRING_DICTIONARY: u16 = 0xFFFF;

// Special value when no palette change is necessary
const NO_PALETTE_CHANGE_REQUESTED: u8 = 0xFF;

const VID_PAGE_SIZE: usize = 320 * 200 / 2;

pub(crate) struct Video {
    // typedef void (Video::*drawLine)(int16_t x1, int16_t x2, uint8_t col);
    res: ResourceRef,
    sys: SystemRef,

    pub palette_id_requested: u8,
    current_palette_id: u8,
    // page_offsets: [usize; 4];

    // I am almost sure that:
    // _curPagePtr1 is the back buffer
    // _curPagePtr2 is the front buffer
    // _curPagePtr3 is the background builder.
    cur_page_idx1: usize,
    cur_page_idx2: usize,
    cur_page_idx3: usize,

    polygon: Polygon,
    hliney: i16,

    //Precomputer division lookup table
    interp_table: [u16; 0x400],

    data_page_idx: usize,
    data: SliceReader,

    pages_buf: [[u8; VID_PAGE_SIZE]; 4],
    mask: u8,
}

impl Video {
    pub fn new(res: ResourceRef, sys: SystemRef) -> Self {
        Self {
            res,
            sys,
            palette_id_requested: 0,
            current_palette_id: 0,
            // page_offsets: [],
            cur_page_idx1: 0,
            cur_page_idx2: 0,
            cur_page_idx3: 0,
            polygon: Polygon::new(),
            hliney: 0,
            interp_table: [0; 0x400],
            data_page_idx: 0,
            data: Default::default(),
            pages_buf: [[0; VID_PAGE_SIZE]; 4],
            mask: 0,
        }
    }

    pub fn init(&mut self) {
        self.palette_id_requested = NO_PALETTE_CHANGE_REQUESTED;

        // TODO: is this needed?
        // self.pages_buf = [[0; VID_PAGE_SIZE]; 4];

        /*
        for (int i = 0; i < 4; ++i) {
            _pagePtrs[i] = allocPage();
        }
        */
        // for (int i = 0; i < 4; ++i) {
        // 	_pagePtrs[i] = tmp + i * VID_PAGE_SIZE;
        // }

        self.cur_page_idx3 = self.get_page_off(1);
        self.cur_page_idx2 = self.get_page_off(2);

        self.change_page_off1(0xFE);

        self.interp_table[0] = 0x4000;

        for i in 1..self.interp_table.len() {
            self.interp_table[i] = (0x4000 / i) as u16;
        }
    }

    pub(crate) fn set_data_page(&mut self, cinematic: bool, offset: usize) {
        self.data_page_idx = if cinematic {
            self.res.get().seg_cinematic_idx()
        } else {
            self.res.get().seg_video2_idx()
        };

        self.data = self.res.get().get_entry_data(self.data_page_idx).into();
        self.data.set_pos(offset);
    }

    // A shape can be given in two different ways:
    // 	- A list of screen space vertices.
    // 	- A list of object space vertices, based on a delta from the first vertex.

    // 	This is a recursive function.
    pub(crate) fn read_and_draw_polygon(&mut self, mut color: u8, zoom: u16, pt: Point) {
        let mut i = self.data.read_u8();

        // 0xc0 = 192
        if i >= 0xC0 {
            // WTF ?
            //0x80 = 128 (1000 0000)
            if color & 0x80 != 0 {
                color = i & 0x3F; //0x3F =  63 (0011 1111)
            }

            // pc is misleading here since we are not reading bytecode but only
            // vertices information.
            let old_pos = self.data.pos();

            self.polygon.read_vertices(&mut self.data, zoom);

            self.data.set_pos(old_pos);

            self.fill_polygon(color, zoom, pt);
        } else {
            i &= 0x3F; //0x3F = 63
            if i == 1 {
                println!("Video::read_and_draw_polygon() ec=0xF80 (i != 2)");
            } else if i == 2 {
                self.read_and_draw_polygon_hierarchy(zoom, pt);
            } else {
                println!("Video::read_and_draw_polygon() ec=0xFBB (i != 2)");
            }
        }
    }

    fn fill_polygon(&mut self, color: u8, _zoom: u16, pt: Point) {
        if self.polygon.bbw == 0 && self.polygon.bbh == 1 && self.polygon.num_points == 4 {
            self.draw_point(color, pt.x, pt.y);

            return;
        }

        let mut x1 = pt.x - (self.polygon.bbw / 2) as i16;
        let mut x2 = pt.x + (self.polygon.bbw / 2) as i16;
        let y1 = pt.y - (self.polygon.bbh / 2) as i16;
        let y2 = pt.y + (self.polygon.bbh / 2) as i16;

        if x1 > 319 || x2 < 0 || y1 > 199 || y2 < 0 {
            return;
        }

        self.hliney = y1;

        let mut i = 0;
        // let mut j = w_sub(self.polygon.num_points, 1) as usize;
        let mut j = (self.polygon.num_points - 1) as usize;

        x2 = self.polygon.points[i].x + x1;
        x1 = self.polygon.points[j].x + x1;

        i += 1;
        j -= 1;

        // let mut draw_func: Box<dyn FnMut(i16, i16, u8)> = if color < 0x10 {
        // 	Box::new(|x1, x2, c| self.draw_line_n(x1, x2, c))
        // } else if color > 0x10 {
        // 	Box::new(|x1, x2, c| self.draw_line_p(x1, x2, c))
        // } else {
        // 	Box::new(|x1, x2, c| self.draw_line_blend(x1, x2, c))
        // };

        let mut cpt1 = (x1 as u32) << 16;
        let mut cpt2 = (x2 as u32) << 16;

        loop {
            self.polygon.num_points -= 2;
            if self.polygon.num_points == 0 {
                // #if TRACE_FRAMEBUFFER
                // 				dumpFrameBuffers("fillPolygonEnd");
                // 		#endif
                // #if TRACE_BG_BUFFER
                // 			dumpBackGroundBuffer();
                // #endif
                break;
            }

            let (step1, _) = self.calc_step(self.polygon.points[j + 1], self.polygon.points[j]);
            let (step2, h) = self.calc_step(self.polygon.points[i - 1], self.polygon.points[i]);

            i += 1;
            j -= 1;

            cpt1 = (cpt1 & 0xFFFF_0000) | 0x7FFF;
            cpt2 = (cpt2 & 0xFFFF_0000) | 0x8000;

            if h == 0 {
                cpt1 += step1 as u32;
                cpt2 += step2 as u32;
            } else {
                for _ in 0..h {
                    if self.hliney >= 0 {
                        x1 = (cpt1 >> 16) as i16;
                        x2 = (cpt2 >> 16) as i16;
                        if x1 <= 319 && x2 >= 0 {
                            if x1 < 0 {
                                x1 = 0;
                            }
                            if x2 > 319 {
                                x2 = 319;
                            }
                            // (*draw_func)(x1, x2, color); // TODO: uncomment
                            match color.cmp(&0x10) {
                                Ordering::Less => self.draw_line_n(x1, x2, color),
                                Ordering::Greater => self.draw_line_p(x1, x2, color),
                                Ordering::Equal => self.draw_line_blend(x1, x2, color),
                            }
                        }
                    }

                    cpt1 = w_add_u32(cpt1, step1 as _);
                    cpt2 = w_add_u32(cpt2, step2 as _);

                    // cpt1 += step1 as u32;
                    // cpt2 += step2 as u32;
                    self.hliney += 1;
                    if self.hliney > 199 {
                        return;
                    }
                }
            }

            // #if TRACE_FRAMEBUFFER
            // 		dumpFrameBuffers("fillPolygonChild");
            // #endif
            // 		#if TRACE_BG_BUFFER

            // 	dumpBackGroundBuffer();
            // #endif
        }
    }

    // What is read from the bytecode is not a pure screen space polygon but a polygon space polygon.
    fn read_and_draw_polygon_hierarchy(&mut self, zoom: u16, pgc: Point) {
        let mut pt = pgc;
        pt.x -= (self.data.read_u8() as u16 * zoom / 64) as i16;
        pt.y -= (self.data.read_u8() as u16 * zoom / 64) as i16;

        let children = self.data.read_u8();
        // debug(DBG_VIDEO, "Video::readAndDrawPolygonHierarchy childs=%d", childs);

        for _ in 0..children {
            let mut off = self.data.read_u16();
            let mut po = pt;

            po.x += (self.data.read_u8() as u16 * zoom / 64) as i16;
            po.y += (self.data.read_u8() as u16 * zoom / 64) as i16;

            let mut color = 0xFF;
            let bp = off;
            off &= 0x7FFF;

            if bp & 0x8000 != 0 {
                color = self.data.read_u8() & 0x7F;
                self.data.read_u8();
            }

            let old_pos = self.data.pos();
            self.data.set_pos((off * 2) as usize);

            self.read_and_draw_polygon(color, zoom, po);

            self.data.set_pos(old_pos);
        }
    }

    fn calc_step(&self, p1: Point, p2: Point) -> (i16, usize) {
        let dy = (p2.y - p1.y) as usize;
        let dx = w_mul_i16(w_mul_i16(p2.x - p1.x, self.interp_table[dy] as i16), 4);
        (dx, dy)
    }

    pub(crate) fn draw_string(&mut self, color: u8, mut x: u16, mut y: u16, string_id: u16) {
        if let Some(se) = STRINGS_TABLE_ENG.get(&string_id) {
            // debug(DBG_VIDEO, "drawString(%d, %d, %d, '%s')", color, x, y, se->str);

            //Used if the string contains a return carriage.
            let x_origin = x;

            for ch in se.chars() {
                if ch == '\n' {
                    y += 8;
                    x = x_origin;
                    continue;
                }

                self.draw_char(ch, x, y, color, self.cur_page_idx1);
                x += 1;
            }
        }
    }

    fn draw_char(&mut self, character: char, x: u16, y: u16, color: u8, idx: usize) {
        if x <= 39 && y <= 192 {
            let font_off = (character as u8 - b' ') as usize * 8;

            let mut buf_off = (x * 4 + y * 160) as usize;

            for j in 0..8 {
                let mut ch = FONT[font_off + j];
                for i in 0..4 {
                    let b = self.pages_buf[idx][buf_off + i];
                    let mut cmask = 0xFF;
                    let mut colb = 0;
                    if ch & 0x80 != 0 {
                        colb |= color << 4;
                        cmask &= 0x0F;
                    }
                    ch <<= 1;
                    if ch & 0x80 != 0 {
                        colb |= color;
                        cmask &= 0xF0;
                    }
                    ch <<= 1;
                    self.pages_buf[idx][buf_off + i] = (b & cmask) | colb;
                }
                buf_off += 160;
            }
        }
    }

    fn draw_point(&mut self, color: u8, x: i16, y: i16) {
        // debug(DBG_VIDEO, "drawPoint(%d, %d, %d)", color, x, y);
        if x >= 0 && x <= 319 && y >= 0 && y <= 199 {
            let off = (y * 160 + x / 2) as usize;

            let (mut cmaskn, mut cmasko) = if x & 1 != 0 {
                (0x0F, 0xF0)
            } else {
                (0xF0, 0x0F)
            };

            // uint8_t colb = (color << 4) | color;
            let colb = if color == 0x10 {
                cmaskn &= 0x88;
                cmasko = !cmaskn;
                0x88
            } else if color == 0x11 {
                self.pages_buf[0][off]
            } else {
                (color << 4) | color
            };
            let b = self.pages_buf[self.cur_page_idx1][off];
            self.pages_buf[self.cur_page_idx1][off] = (b & cmasko) | (colb & cmaskn);
        }
    }

    // Blend a line in the current framebuffer (_curPagePtr1)
    fn draw_line_blend(&mut self, x1: i16, x2: i16, _color: u8) {
        // debug(DBG_VIDEO, "drawLineBlend(%d, %d, %d)", x1, x2, color);
        let xmax = std::cmp::max(x1, x2);
        let xmin = std::cmp::min(x1, x2);
        let mut off = (self.hliney * 160 + xmin / 2) as usize;
        let mut w = xmax / 2 - xmin / 2 + 1;

        let cmasks = if xmin & 1 != 0 {
            w -= 1;
            0xF7
        } else {
            0
        };

        let cmaske = if xmax & 1 == 0 {
            w -= 1;
            0x7F
        } else {
            0
        };

        if cmasks != 0 {
            self.pages_buf[self.cur_page_idx1][off] =
                (self.pages_buf[self.cur_page_idx1][off] & cmasks) | 0x08;
            off += 1;
        }

        for _ in 0..w {
            self.pages_buf[self.cur_page_idx1][off] =
                (self.pages_buf[self.cur_page_idx1][off] & 0x77) | 0x88;
            off += 1;
        }

        if cmaske != 0 {
            self.pages_buf[self.cur_page_idx1][off] =
                (self.pages_buf[self.cur_page_idx1][off] & cmaske) | 0x80;
            // off += 1;
        }
    }

    fn draw_line_n(&mut self, x1: i16, x2: i16, color: u8) {
        // debug(DBG_VIDEO, "drawLineN(%d, %d, %d)", x1, x2, color);
        let xmax = std::cmp::max(x1, x2);
        let xmin = std::cmp::min(x1, x2);
        let mut off = (self.hliney * 160 + xmin / 2) as usize;
        let mut w = xmax / 2 - xmin / 2 + 1;

        let cmasks = if xmin & 1 != 0 {
            w -= 1;
            0xF0
        } else {
            0
        };

        let cmaske = if xmax & 1 == 0 {
            w -= 1;
            0x0F
        } else {
            0
        };

        let colb = ((color & 0xF) << 4) | (color & 0xF);
        if cmasks != 0 {
            self.pages_buf[self.cur_page_idx1][off] =
                (self.pages_buf[self.cur_page_idx1][off] & cmasks) | (colb & 0x0F);
            off += 1;
        }
        for _ in 0..w {
            self.pages_buf[self.cur_page_idx1][off] = colb;
            off += 1;
        }
        if cmaske != 0 {
            self.pages_buf[self.cur_page_idx1][off] =
                (self.pages_buf[self.cur_page_idx1][off] & cmaske) | (colb & 0xF0);
            // off += 1;
        }
    }

    fn draw_line_p(&mut self, x1: i16, x2: i16, _color: u8) {
        // debug(DBG_VIDEO, "drawLineP(%d, %d, %d)", x1, x2, color);
        let xmax = std::cmp::max(x1, x2);
        let xmin = std::cmp::min(x1, x2);
        let mut off = (self.hliney * 160 + xmin / 2) as usize;
        let mut w = xmax / 2 - xmin / 2 + 1;

        let cmasks = if xmin & 1 != 0 {
            w -= 1;
            0xF0
        } else {
            0
        };

        let cmaske = if xmax & 1 == 0 {
            w -= 1;
            0x0F
        } else {
            0
        };

        if cmasks != 0 {
            self.pages_buf[self.cur_page_idx1][off] = (self.pages_buf[self.cur_page_idx1][off]
                & cmasks)
                | (self.pages_buf[0][off] & 0x0F);
            off += 1;
        }
        for _ in 0..w {
            self.pages_buf[self.cur_page_idx1][off] = self.pages_buf[0][off];
            off += 1;
        }
        if cmaske != 0 {
            self.pages_buf[self.cur_page_idx1][off] = (self.pages_buf[self.cur_page_idx1][off]
                & cmaske)
                | (self.pages_buf[0][off] & 0xF0);
            // off += 1;
        }
    }

    fn get_page_off(&self, page_id: usize) -> usize {
        if page_id < self.pages_buf.len() {
            page_id
        } else if page_id == 0xFF {
            self.cur_page_idx3
        } else if page_id == 0xFE {
            self.cur_page_idx2
        } else {
            // warning("Video::getPagePtr() p != [0,1,2,3,0xFF,0xFE] == 0x%X", page);
            0 // XXX check
        }
    }

    pub(crate) fn change_page_off1(&mut self, page: usize) {
        // debug(DBG_VIDEO, "Video::changePagePtr1(%d)", page);
        self.cur_page_idx1 = self.get_page_off(page);
    }

    pub(crate) fn fill_page(&mut self, page: usize, color: u8) {
        // debug(DBG_VIDEO, "Video::fillPage(%d, %d)", page, color);
        let page_off = self.get_page_off(page);

        // Since a palette indice is coded on 4 bits, we need to duplicate the
        // clearing color to the upper part of the byte.
        let c = (color << 4) | color;

        self.pages_buf[page_off] = [c; VID_PAGE_SIZE];

        // #if TRACE_FRAMEBUFFER
        // 			dumpFrameBuffers("-fillPage");
        // #endif
        // 			#if TRACE_BG_BUFFER

        // 			dumpBackGroundBuffer();
        // #endif
    }

    // This opcode is used once the background of a scene has been drawn in one of the framebuffer:
    // it is copied in the current framebuffer at the start of a new frame in order to improve performances.
    pub(crate) fn copy_page(&mut self, src_page_id: usize, dst_page_id: usize, vscroll: i16) {
        // debug(DBG_VIDEO, "Video::copyPage(%d, %d)", srcPageId, dstPageId);

        if src_page_id == dst_page_id {
            return;
        }

        let src_mask = src_page_id & 0xBF;
        let mut q = self.get_page_off(dst_page_id);

        if src_page_id >= 0xFE {
            let p = self.get_page_off(src_page_id);

            self.pages_buf[q] = self.pages_buf[p];
        } else if (src_mask & 0x80) == 0 {
            let p = self.get_page_off(src_mask);

            self.pages_buf[q] = self.pages_buf[p];
        } else {
            let mut p = self.get_page_off(src_mask & 3);

            if vscroll >= -199 && vscroll <= 199 {
                let mut h = 200;

                if vscroll < 0 {
                    h += vscroll as usize;
                    p += -vscroll as usize * 160;
                } else {
                    h -= vscroll as usize;
                    q += vscroll as usize * 160;
                }

                if p < q {
                    let (p_arr, q_arr) = self.pages_buf.split_at_mut(q);
                    q_arr[0][..h * 160].clone_from_slice(&p_arr[p][..h * 160]);
                } else if q < p {
                    let (q_arr, p_arr) = self.pages_buf.split_at_mut(p);
                    q_arr[q][..h * 160].clone_from_slice(&p_arr[0][..h * 160]);
                } else {
                    // TODO: error
                    todo!();
                }
            }
        }

        // #if TRACE_FRAMEBUFFER
        // char name[256];
        // memset(name,0,sizeof(name));
        // sprintf(name,"copyPage_0x%X_to_0x%X",(p-_pagePtrs[0])/VID_PAGE_SIZE,(q-_pagePtrs[0])/VID_PAGE_SIZE);
        // dumpFrameBuffers(name);
        // #endif
    }

    fn copy_page_data(&mut self, src: &mut [u8]) {
        // debug(DBG_VIDEO, "Video::copyPagePtr()");
        let mut src_idx = 0;
        let mut dst_idx = 0;

        for _ in 0..200 {
            for _ in 0..40 {
                let p = [
                    src_idx + 8000 * 3,
                    src_idx + 8000 * 2,
                    src_idx + 8000 * 1,
                    src_idx, // + 8000 * 0,
                ];
                for _ in 0..4 {
                    let mut acc = 0;
                    for i in 0..8 {
                        acc <<= 1;
                        acc |= if (src[p[i] & 3] & 0x80) != 0 { 1 } else { 0 };
                        src[p[i] & 3] <<= 1;
                    }

                    self.pages_buf[0][dst_idx] = acc;
                    dst_idx += 1;
                }
                src_idx += 1;
            }
        }
    }

    // uint8_t *allocPage();

    /*
    Note: The palettes set used to be allocated on the stack but I moved it to
        the heap so I could dump the four framebuffer and follow how
        frames are generated.
    */
    fn change_pal(&mut self, pal_num: usize) {
        if pal_num >= 32 {
            return;
        }

        let mut pal_idx = pal_num * 32; //colors are coded on 2bytes (565) for 16 colors = 32
                                        // res->segPalettes

        // Moved to the heap, legacy code used to allocate the palette
        // on the stack.
        let mut palette = [0u8; NUM_COLORS * BYTE_PER_PIXEL];
        let res = self.res.get();

        for i in 0..NUM_COLORS {
            let c = res.read_palette(pal_idx, 2);

            pal_idx += 2;

            palette[i * 3 + 0] = ((c[0] & 0x0F) << 2) | ((c[0] & 0x0F) >> 2); // r
            palette[i * 3 + 1] = ((c[1] & 0xF0) >> 2) | ((c[1] & 0xF0) >> 6); // g
            palette[i * 3 + 2] = ((c[1] & 0x0F) >> 2) | ((c[1] & 0x0F) << 2); // b
        }

        self.sys
            .get_mut()
            .set_palette(0, NUM_COLORS as u8, &palette);
        self.current_palette_id = pal_num as u8;

        // #if TRACE_PALETTE
        // printf("\nuint8_t dumpPalette[48] = {\n");
        // for (int i = 0; i < NUM_COLORS; ++i)
        // {
        // 	printf("0x%X,0x%X,0x%X,",pal[i * 3 + 0],pal[i * 3 + 1],pal[i * 3 + 2]);
        // }
        // printf("\n};\n");
        // #endif

        // #if TRACE_FRAMEBUFFER
        // 	dumpPaletteCursor++;
        // #endif
    }

    pub(crate) fn update_display(&mut self, page: usize) {
        // debug(DBG_VIDEO, "Video::updateDisplay(%d)", pageId);

        if page != 0xFE {
            if page == 0xFF {
                std::mem::swap(&mut self.cur_page_idx2, &mut self.cur_page_idx3);
            } else {
                self.cur_page_idx2 = self.get_page_off(page);
            }
        }

        //Check if we need to change the palette
        if self.palette_id_requested != NO_PALETTE_CHANGE_REQUESTED {
            self.change_pal(self.palette_id_requested as usize);
            self.palette_id_requested = NO_PALETTE_CHANGE_REQUESTED
        }

        //Q: Why 160 ?
        //A: Because one byte gives two palette indices so
        //   we only need to move 320/2 per line.
        self.sys
            .get_mut()
            .copy_rect(0, 0, 320, 200, &self.pages_buf[self.cur_page_idx2][..], 160);

        // #if TRACE_FRAMEBUFFER
        // 	  dumpFrameBuffer(_curPagePtr2,allFrameBuffers,320,200);
        // #endif
    }

    pub fn save_or_load(&mut self, ser: &mut Serializer) -> Result<()> {
        self.mask = 0;

        if ser.mode() == Mode::Save {
            for i in 0..4 {
                if i == self.cur_page_idx1 {
                    self.mask |= (i << 4) as u8;
                }
                if i == self.cur_page_idx2 {
                    self.mask |= (i << 2) as u8;
                }
                if i == self.cur_page_idx3 {
                    self.mask |= (i << 0) as u8;
                }
            }
        }

        ser.save_or_load_entries(self, Ver(1))?;

        if ser.mode() == Mode::Load {
            self.cur_page_idx1 = ((self.mask >> 4) & 0x3) as usize;
            self.cur_page_idx2 = ((self.mask >> 2) & 0x3) as usize;
            self.cur_page_idx3 = ((self.mask >> 0) & 0x3) as usize;
            self.change_pal(self.current_palette_id as usize);
        }

        Ok(())
    }

    // #define TRACE_PALETTE 0
    // #define TRACE_FRAMEBUFFER 0
    // #if TRACE_FRAMEBUFFER
    //     void dumpFrameBuffer(uint8_t *src,uint8_t *dst, int x,int y);
    // 	void dumpFrameBuffers(char* comment);

    // #endif

    // #define TRACE_BG_BUFFER 0
    // #if TRACE_BG_BUFFER
    // 	void dumpBackGroundBuffer();
    // #endif
}

// TODO: use proc_macro

impl AccessorWrap for Video {
    fn read(&mut self, stream: &mut File) -> Result<()> {
        self.current_palette_id.read(stream)?;
        self.palette_id_requested.read(stream)?;
        self.mask.read(stream)?;
        self.pages_buf.read(stream)
    }

    fn write(&self, stream: &mut File) -> Result<()> {
        self.current_palette_id.write(stream)?;
        self.palette_id_requested.write(stream)?;
        self.mask.write(stream)?;
        self.pages_buf.write(stream)
    }

    fn size(&self) -> usize {
        self.current_palette_id.size()
            + self.palette_id_requested.size()
            + self.mask.size()
            + self.pages_buf.size()
    }
}
