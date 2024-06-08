mod error;
mod record;

use std::cell::RefCell;
use std::io::{self, Read};
use std::rc::Rc;

use error::OmfError;
use num_traits::FromPrimitive;
use record::{
    AbsoluteSegmentAddress, CommentType, ExtName, GroupComponent, MAttrStart, OmfRecord,
    OmfRecordData, PubName, SegmentAlignment, SegmentAttributes,
};

#[derive(Debug, Clone)]
pub struct SegmentInfo {
    pub segment_attributes: SegmentAttributes,
    pub segment_length: u16,
    pub segment_name_index: u8,
    pub class_name_index: u8,
    pub overlay_name_index: u8,
}

#[derive(Debug, Clone)]
pub struct GroupInfo {
    pub group_name_index: u8,
    pub segment_definitions: Vec<GroupComponent>,
}

#[derive(Debug)]
pub struct OmfInfo {
    pub names: Vec<String>,
    pub segments: Vec<SegmentInfo>,
    pub groups: Vec<GroupInfo>,
}

impl OmfInfo {
    pub fn new() -> OmfInfo {
        OmfInfo {
            names: vec![],
            segments: vec![],
            groups: vec![],
        }
    }
}

pub struct OmfReader<'a> {
    r: &'a mut dyn Read,
    info: Rc<RefCell<OmfInfo>>,
}

impl<'a> OmfReader<'a> {
    pub fn new(r: &'a mut dyn Read) -> OmfReader<'a> {
        OmfReader {
            r,
            info: Rc::new(RefCell::new(OmfInfo::new())),
        }
    }

    fn read_u8(&mut self) -> Result<u8, io::Error> {
        let mut buf = [0u8; 1];
        self.r.read_exact(&mut buf)?;
        Ok(buf[0])
    }

    fn read_u16(&mut self) -> Result<u16, io::Error> {
        let mut buf = [0u8; 2];
        self.r.read_exact(&mut buf)?;
        Ok(u16::from_le_bytes(buf))
    }

    fn read_bytes(&mut self, len: usize) -> Result<Vec<u8>, io::Error> {
        let mut buf = Vec::new();
        buf.resize(len, 0u8);
        self.r.read_exact(&mut buf)?;
        Ok(buf)
    }

    fn read_string(&mut self) -> Result<String, OmfError> {
        let len = self.read_u8()? as usize;
        let str_vec = self.read_bytes(len)?;
        Ok(String::from_utf8(str_vec)?)
    }

    fn get_next_record(&mut self) -> Result<Option<OmfRecord>, OmfError> {
        let record_type = match self.read_u8() {
            Ok(v) => v,
            Err(_) => return Ok(None),
        };
        let record_length = self.read_u16()? as usize;

        let data = match record_type {
            0x80 => {
                let name = self.read_string()?;
                OmfRecordData::THeadr { name }
            }
            0x88 => {
                let tmp = self.read_u8()?;
                let no_purge = tmp & 0x80 != 0;
                let no_list = tmp & 0x80 != 0;
                let comment_type = CommentType { no_purge, no_list };
                let comment_class = self.read_u8()?;
                let comment_bytes = self.read_bytes(record_length - 3)?;
                OmfRecordData::Coment {
                    comment_type,
                    comment_class,
                    comment_bytes,
                }
            }
            0x8A => {
                let module_type = self.read_u8()?;
                let main = module_type & 0x80 != 0;
                let start = if module_type & 0x40 != 0 {
                    MAttrStart::Start {
                        end_data: self.read_u8()?,
                        frame_datum: self.read_u8()?,
                        target_datum: self.read_u8()?,
                        target_displacement: self.read_u16()?,
                    }
                } else {
                    MAttrStart::NoStart
                };
                OmfRecordData::ModEnd { main, start }
            }
            0x8C => {
                let mut names = vec![];
                let mut c = 0;
                while c < record_length - 1 {
                    let name = self.read_string()?;
                    let type_index = self.read_u8()?;
                    c += name.len() + 2;
                    names.push(ExtName { name, type_index });
                }
                OmfRecordData::ExtDef { names }
            }
            0x90 => {
                let base_group_index = self.read_u8()?;
                let base_segment_index = self.read_u8()?;
                let base_frame = if base_segment_index == 0 {
                    self.read_u16()?
                } else {
                    0u16
                };
                let mut names = vec![];
                let mut c = 0;
                let rep_len = record_length - 3 - if base_segment_index == 0 { 2 } else { 0 };
                while c < rep_len {
                    let name = self.read_string()?;
                    let public_offset = self.read_u16()?;
                    let type_index = self.read_u8()?;
                    c += name.len() + 4;
                    names.push(PubName {
                        name,
                        public_offset,
                        type_index,
                    });
                }
                OmfRecordData::PubDef {
                    base_group_index,
                    base_segment_index,
                    base_frame,
                    names,
                }
            }
            0x96 => {
                let mut names = vec![];
                let mut c = 0;
                while c < record_length - 1 {
                    let name = self.read_string()?;
                    c += name.len() + 1;
                    names.push(name);
                }
                self.info.borrow_mut().names.append(&mut (names.clone()));
                OmfRecordData::LNames { names }
            }
            0x98 => {
                let tmp = self.read_u8()?;
                let alignment =
                    FromPrimitive::from_u8(tmp >> 5).ok_or(OmfError::Value("alignment"))?;
                let combination =
                    FromPrimitive::from_u8((tmp >> 2) & 3).ok_or(OmfError::Value("combination"))?;
                let absolute_segment_address = if alignment == SegmentAlignment::AbsoluteSegment {
                    let frame_number = self.read_u16()?;
                    let offset = self.read_u8()?;
                    Some(AbsoluteSegmentAddress {
                        frame_number,
                        offset,
                    })
                } else {
                    None
                };
                let segment_attributes = SegmentAttributes {
                    alignment,
                    combination,
                    big: tmp & 2 != 0,
                    bd32bit: tmp & 1 != 0,
                    absolute_segment_address,
                };
                let segment_length = self.read_u16()?;
                let segment_name_index = self.read_u8()?;
                let class_name_index = self.read_u8()?;
                let overlay_name_index = self.read_u8()?;
                self.info.borrow_mut().segments.push(SegmentInfo {
                    segment_attributes: segment_attributes,
                    segment_length,
                    segment_name_index,
                    class_name_index,
                    overlay_name_index,
                });
                OmfRecordData::SegDef {
                    segment_attributes,
                    segment_length,
                    segment_name_index,
                    class_name_index,
                    overlay_name_index,
                }
            }
            0x9A => {
                let group_name_index = self.read_u8()?;
                let mut segment_definitions = vec![];
                let mut c = 0;
                while c < record_length - 2 {
                    let index = self.read_u8()?;
                    let segment_definition = self.read_u8()?;
                    c += 2;
                    segment_definitions.push(GroupComponent {
                        index,
                        segment_definition,
                    });
                }
                self.info.borrow_mut().groups.push(GroupInfo {
                    group_name_index,
                    segment_definitions: segment_definitions.clone(),
                });
                OmfRecordData::GrpDef {
                    group_name_index,
                    segment_definitions,
                }
            }
            0xA0 => {
                let segment_index = self.read_u8()?;
                let enumerated_data_offset = self.read_u16()?;
                let data = self.read_bytes(record_length - 4)?;
                OmfRecordData::LEData {
                    segment_index,
                    enumerated_data_offset,
                    data,
                }
            }
            _ => {
                let data = self.read_bytes(record_length - 1)?;
                OmfRecordData::Unknown { data }
            }
        };
        let checksum = self.read_u8()?;

        Ok(Some(OmfRecord::new(
            record_type,
            record_length,
            data,
            checksum,
            Rc::clone(&self.info),
        )))
    }
}

impl<'a> Iterator for OmfReader<'a> {
    type Item = OmfRecord;

    fn next(&mut self) -> Option<Self::Item> {
        self.get_next_record().expect("next")
    }
}
