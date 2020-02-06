use crate::file::File;
use crate::resource::*;
use crate::serializer::*;
use crate::system::*;
use anyhow::Result;

struct StrEntry {
	id: u16,
	val: String,
}

#[derive(Clone, Copy, Default)]
struct Point {
    x: i16,
    y: i16,
}

impl Point {
	fn new(x: i16, y: i16) -> Self {
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

	fn read_vertices(&mut self, buf: &[u8], zoom: u16) {
		todo!(); // TODO: implement
	}
}

// This is used to detect the end of  _stringsTableEng and _stringsTableDemo
pub const END_OF_STRING_DICTIONARY: u16 = 0xFFFF;

// Special value when no palette change is necessary
const NO_PALETTE_CHANGE_REQUESTED: u8 = 0xFF;

const VID_PAGE_SIZE: usize = 320 * 200 / 2;


struct Video {
	// typedef void (Video::*drawLine)(int16_t x1, int16_t x2, uint8_t col);

	res: ResourceRef,
	sys: SystemRef,

	palette_id_requested: u8,
	current_palette_id: u8,
	// page_offsets: [usize; 4];

	// I am almost sure that:
	// _curPagePtr1 is the backbuffer 
	// _curPagePtr2 is the frontbuffer
	// _curPagePtr3 is the background builder.
	cur_page_idx1: usize,
	cur_page_idx2: usize,
	cur_page_idx3: usize,

	polygon: Polygon,
	hliney: u16,

	//Precomputer division lookup table
	interp_table: [u16; 0x400],

	// Ptr _pData;
	// uint8_t *_dataBuf;

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
			pages_buf: [[0; VID_PAGE_SIZE]; 4],
			mask: 0,
        }
    }

	fn init(&mut self) {
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

	// void setDataBuffer(uint8_t *dataBuf, uint16_t offset);
	// void readAndDrawPolygon(uint8_t color, uint16_t zoom, const Point &pt);
	// void fillPolygon(uint16_t color, uint16_t zoom, const Point &pt);
	// void readAndDrawPolygonHierarchy(uint16_t zoom, const Point &pt);
	// int32_t calcStep(const Point &p1, const Point &p2, uint16_t &dy);

	// void drawString(uint8_t color, uint16_t x, uint16_t y, uint16_t strId);
	// void drawChar(uint8_t c, uint16_t x, uint16_t y, uint8_t color, uint8_t *buf);
	// void drawPoint(uint8_t color, int16_t x, int16_t y);
	// void drawLineBlend(int16_t x1, int16_t x2, uint8_t color);
	// void drawLineN(int16_t x1, int16_t x2, uint8_t color);
	// void drawLineP(int16_t x1, int16_t x2, uint8_t color);

	fn get_page_off(&self, page: usize) -> usize {
		if page < self.pages_buf.len() {
			page
		} else if page == 0xFF {
			self.cur_page_idx3
		} else if page == 0xFE {
			self.cur_page_idx2
		} else {
			// warning("Video::getPagePtr() p != [0,1,2,3,0xFF,0xFE] == 0x%X", page);
			0 // XXX check
		}
	}

	fn change_page_off1(&mut self, page: usize) {
		// debug(DBG_VIDEO, "Video::changePagePtr1(%d)", page);
		self.cur_page_idx1 = self.get_page_off(page);
	}

	fn fill_page(&mut self, page: usize, color: u8) {
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

	/*  This opcode is used once the background of a scene has been drawn in one of the framebuffer:
	   it is copied in the current framebuffer at the start of a new frame in order to improve performances. */
	fn copy_page(&mut self, src_page: usize, dst_page: usize, vscroll: i16) {

		// debug(DBG_VIDEO, "Video::copyPage(%d, %d)", srcPageId, dstPageId);
	
		if src_page == dst_page {
			return;
		}
	
		let src_mask = src_page & 0xBF;
		let mut q = self.get_page_off(dst_page);

		if src_page >= 0xFE {
			let p = self.get_page_off(src_page);

			self.pages_buf[q] = self.pages_buf[p].clone();
		} else if (src_mask & 0x80) == 0 {
			let p = self.get_page_off(src_mask);

			self.pages_buf[q] = self.pages_buf[p].clone();
		} else {
			let mut p = self.get_page_off(src_page & 3);

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

	fn copy_page_data(&mut self, src: &[u8]) {
		// debug(DBG_VIDEO, "Video::copyPagePtr()");
		let mut src_idx = 0;
		let mut dst_idx = 0;

		for _ in 0..200 {
			for _ in 0..40 {
				let mut p = [
					src_idx + 8000 * 3,
					src_idx + 8000 * 2,
					src_idx + 8000 * 1,
					src_idx + 8000 * 0,
				];
				for _ in 0..4 {
					let mut acc = 0;
					for i in 0..8 {
						acc <<= 1;
						acc |= if (p[i & 3] & 0x80) != 0 { 1 } else { 0 };
						p[i & 3] <<= 1;
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

		self.sys.get_mut().set_palette(0, NUM_COLORS as u8, &palette);
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

	fn update_display(&mut self, page: usize) {
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
		self.sys.get_mut().copy_rect(0, 0, 320, 200, &self.pages_buf[self.cur_page_idx2][..], 160);
	
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
        // self.mask.read(stream)?; // TODO:
        self.pages_buf[0].read(stream)?;
        self.pages_buf[1].read(stream)?;
        self.pages_buf[2].read(stream)?;
        self.pages_buf[3].read(stream)
    }

    fn write(&self, stream: &mut File) -> Result<()> {
        self.current_palette_id.write(stream)?;
        self.palette_id_requested.write(stream)?;
        // self.mask.write(stream)?; // TODO:
        self.pages_buf[0].write(stream)?;
        self.pages_buf[1].write(stream)?;
        self.pages_buf[2].write(stream)?;
        self.pages_buf[3].write(stream)
    }

    fn size(&self) -> usize {
        // self.cur_pos.size() + self.cur_order.size()
        self.current_palette_id.size() +
        self.palette_id_requested.size() +
        // self.mask.size() + // TODO:
        self.pages_buf[0].size() +
        self.pages_buf[1].size() +
        self.pages_buf[2].size() +
        self.pages_buf[3].size()
    }
}
