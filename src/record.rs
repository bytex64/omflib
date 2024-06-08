use std::{cell::RefCell, fmt::Display, rc::Rc};

use num_derive::FromPrimitive;
use pretty_hex::{HexConfig, PrettyHex};

use crate::{error::OmfError, GroupInfo, OmfInfo, SegmentInfo};

#[derive(Debug)]
pub struct CommentType {
    pub no_purge: bool,
    pub no_list: bool,
}

#[derive(Debug)]
pub enum MAttrStart {
    NoStart,
    Start {
        end_data: u8,
        frame_datum: u8,
        target_datum: u8,
        target_displacement: u16,
    },
}

#[derive(Debug)]
pub struct PubName {
    pub name: String,
    pub public_offset: u16,
    pub type_index: u8,
}

#[derive(Debug)]
pub struct ExtName {
    pub name: String,
    pub type_index: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, FromPrimitive)]
pub enum SegmentAlignment {
    AbsoluteSegment = 0,
    RelocatableByteAligned = 1,
    RelocatableWordAligned = 2,
    RelocatableParagraphAligned = 3,
    RelocatablePageAligned = 4,
    RelocatableDWordAligned = 5,
}

impl Display for SegmentAlignment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SegmentAlignment::AbsoluteSegment => write!(f, "absolute segment"),
            SegmentAlignment::RelocatableByteAligned => write!(f, "relocatable, byte aligned"),
            SegmentAlignment::RelocatableWordAligned => write!(f, "relocatable, word aligned"),
            SegmentAlignment::RelocatableParagraphAligned => {
                write!(f, "relocatable, paragraph aligned")
            }
            SegmentAlignment::RelocatablePageAligned => write!(f, "relocatable, page aligned"),
            SegmentAlignment::RelocatableDWordAligned => {
                write!(f, "relocatable, double word aligned")
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, FromPrimitive)]
pub enum SegmentCombination {
    Private = 0,
    Public = 2,
    Public2 = 4,
    Stack = 5,
    Common = 6,
    Public3 = 7,
}

impl Display for SegmentCombination {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SegmentCombination::Private => write!(f, "private"),
            SegmentCombination::Public => write!(f, "public"),
            SegmentCombination::Public2 => write!(f, "public"),
            SegmentCombination::Stack => write!(f, "stack"),
            SegmentCombination::Common => write!(f, "common"),
            SegmentCombination::Public3 => write!(f, "public"),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct AbsoluteSegmentAddress {
    pub frame_number: u16,
    pub offset: u8,
}

#[derive(Debug, Clone, Copy)]
pub struct SegmentAttributes {
    pub alignment: SegmentAlignment,
    pub combination: SegmentCombination,
    pub big: bool,
    pub bd32bit: bool,
    pub absolute_segment_address: Option<AbsoluteSegmentAddress>,
}

#[derive(Debug, Clone, Copy)]
pub struct GroupComponent {
    pub index: u8,
    pub segment_definition: u8,
}

#[derive(Debug)]
pub struct OmfRecord {
    pub record_type: u8,
    pub record_length: usize,
    pub data: OmfRecordData,
    pub checksum: u8,
    info: Rc<RefCell<OmfInfo>>,
}

impl OmfRecord {
    pub fn new(
        record_type: u8,
        record_length: usize,
        data: OmfRecordData,
        checksum: u8,
        info: Rc<RefCell<OmfInfo>>,
    ) -> OmfRecord {
        OmfRecord {
            record_type,
            record_length,
            data,
            checksum,
            info,
        }
    }

    pub fn name_from_index(&self, index: u8) -> Result<String, OmfError> {
        let i = (index as usize) - 1;
        let info = self.info.borrow();
        let s = info
            .names
            .get(i)
            .ok_or_else(|| OmfError::Value("name index not found"))?;
        Ok(s.clone())
    }

    pub fn get_segment(&self, index: u8) -> Result<SegmentInfo, OmfError> {
        let i = (index as usize) - 1;
        let info = self.info.borrow();
        let s = info
            .segments
            .get(i)
            .ok_or_else(|| OmfError::Value("segment index not found"))?;
        Ok(s.clone())
    }

    pub fn get_group(&self, index: u8) -> Result<GroupInfo, OmfError> {
        let i = (index as usize) - 1;
        let info = self.info.borrow();
        let s = info
            .groups
            .get(i)
            .ok_or_else(|| OmfError::Value("group index not found"))?;
        Ok(s.clone())
    }
}

impl Display for OmfRecord {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let cfg = HexConfig {
            group: 8,
            ..HexConfig::default()
        };

        writeln!(
            f,
            "Record type {:02X}h length {}",
            self.record_type, self.record_length
        )?;

        match &self.data {
            OmfRecordData::THeadr { name } => {
                writeln!(f, "Translator Header:")?;
                writeln!(f, "    Name: {name}")
            }
            OmfRecordData::Coment {
                comment_type,
                comment_class,
                comment_bytes,
            } => {
                writeln!(
                    f,
                    "Comment - {}{}class {:02X}",
                    if comment_type.no_purge {
                        "no purge "
                    } else {
                        ""
                    },
                    if comment_type.no_list { "no list " } else { "" },
                    comment_class
                )?;
                writeln!(f, "{:?}", comment_bytes.hex_conf(cfg))
            }
            OmfRecordData::ModEnd { main, start } => {
                writeln!(f, "Module End{}", if *main { " (MAIN)" } else { "" })?;
                match start {
                    MAttrStart::NoStart => (),
                    MAttrStart::Start {
                        end_data,
                        frame_datum,
                        target_datum,
                        target_displacement,
                    } => {
                        writeln!(f, "    End data: {end_data:02X}, frame: {frame_datum:02X}, target: {target_datum:02X}, displacement: {target_displacement:04X}")?;
                    }
                }
                Ok(())
            }
            OmfRecordData::ExtDef { names } => {
                writeln!(f, "External Names Definition")?;
                for (i, n) in names.iter().enumerate() {
                    writeln!(f, "    {i:<4} {} type {}", n.name, n.type_index)?;
                }
                Ok(())
            }
            OmfRecordData::PubDef {
                base_group_index,
                base_segment_index,
                base_frame,
                names,
            } => {
                writeln!(f, "Public Names Definition")?;
                if *base_group_index == 0 && *base_segment_index == 0 {
                    writeln!(f, "    Base Frame: {base_frame:04X}")?;
                } else {
                    if *base_group_index == 0 {
                        writeln!(f, "    Base Group: None")?;
                    } else {
                        let base_group = self.get_group(*base_group_index).expect("group index");
                        writeln!(
                            f,
                            "    Base Group: {} ({})",
                            self.name_from_index(base_group.group_name_index)
                                .expect("name index"),
                            base_group_index
                        )?;
                    }
                    let base_segment = self
                        .get_segment(*base_segment_index)
                        .expect("segment index");
                    writeln!(
                        f,
                        "    Base Segment: {} ({})",
                        self.name_from_index(base_segment.segment_name_index)
                            .expect("name lookup"),
                        base_segment_index
                    )?;
                }
                writeln!(f, "    Names:")?;
                for n in names {
                    writeln!(
                        f,
                        "        {} offset {:04X} type {}",
                        n.name, n.public_offset, n.type_index
                    )?;
                }
                Ok(())
            }
            OmfRecordData::LNames { names } => {
                writeln!(f, "List of Names")?;
                for (i, n) in names.iter().enumerate() {
                    writeln!(f, "    {:<4} {}", i + 1, n)?;
                }
                Ok(())
            }
            OmfRecordData::SegDef {
                segment_attributes,
                segment_length,
                segment_name_index,
                class_name_index,
                overlay_name_index,
            } => {
                writeln!(
                    f,
                    "Segment Definition - {} ({})",
                    self.name_from_index(*segment_name_index)
                        .expect("name lookup"),
                    *segment_name_index
                )?;
                writeln!(
                    f,
                    "    Attributes: {}; {} combination{}{}",
                    segment_attributes.alignment,
                    segment_attributes.combination,
                    if segment_attributes.big { "; BIG" } else { "" },
                    if segment_attributes.bd32bit {
                        "; USE32"
                    } else {
                        ""
                    }
                )?;
                writeln!(f, "    Segment length: {segment_length:04X}")?;
                writeln!(
                    f,
                    "    Class name: {} ({})",
                    self.name_from_index(*class_name_index)
                        .expect("name lookup"),
                    class_name_index
                )?;
                writeln!(
                    f,
                    "    Overlay name: {} ({})",
                    self.name_from_index(*overlay_name_index)
                        .expect("name lookup"),
                    overlay_name_index
                )?;
                Ok(())
            }
            OmfRecordData::GrpDef {
                group_name_index,
                segment_definitions,
            } => {
                writeln!(
                    f,
                    "Group Definition - {} ({})",
                    self.name_from_index(*group_name_index)
                        .expect("name lookup"),
                    group_name_index
                )?;
                writeln!(f, "    Segments:")?;
                for (i, s) in segment_definitions.iter().enumerate() {
                    let segment = self
                        .get_segment(s.segment_definition)
                        .expect("segment index");
                    writeln!(
                        f,
                        "        {:<4} {} ({})",
                        i,
                        self.name_from_index(segment.segment_name_index)
                            .expect("name lookup"),
                        s.segment_definition
                    )?;
                }
                Ok(())
            }
            OmfRecordData::LEData {
                segment_index,
                enumerated_data_offset,
                data,
            } => {
                let segment = self.get_segment(*segment_index).expect("segment index");
                writeln!(
                    f,
                    "Logical Enumerated Data - {} ({}) offset {:04X}h",
                    self.name_from_index(segment.segment_name_index)
                        .expect("name lookup"),
                    segment_index,
                    enumerated_data_offset
                )?;
                writeln!(f, "{:?}", data.hex_conf(cfg))
            }
            OmfRecordData::Unknown { data } => {
                writeln!(f, "Unknown Data")?;
                writeln!(f, "{:?}", data.hex_conf(cfg))
            }
        }
    }
}

#[derive(Debug)]
pub enum OmfRecordData {
    THeadr {
        // 80
        name: String,
    },
    Coment {
        // 88
        comment_type: CommentType,
        comment_class: u8,
        comment_bytes: Vec<u8>,
    },
    ModEnd {
        // 8A
        main: bool,
        start: MAttrStart,
    },
    ExtDef {
        // 8C
        names: Vec<ExtName>,
    },
    PubDef {
        // 90
        base_group_index: u8,
        base_segment_index: u8,
        base_frame: u16,
        names: Vec<PubName>,
    },
    LNames {
        // 96
        names: Vec<String>,
    },
    SegDef {
        // 98
        segment_attributes: SegmentAttributes,
        segment_length: u16,
        segment_name_index: u8,
        class_name_index: u8,
        overlay_name_index: u8,
    },
    GrpDef {
        // 9A
        group_name_index: u8,
        segment_definitions: Vec<GroupComponent>,
    },
    LEData {
        // A0
        segment_index: u8,
        enumerated_data_offset: u16,
        data: Vec<u8>,
    },
    Unknown {
        data: Vec<u8>,
    },
}
