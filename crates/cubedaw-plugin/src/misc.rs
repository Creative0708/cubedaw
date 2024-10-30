use wasm_encoder::reencode::Reencode;

use super::{
    instructions::PreparedInstructionList,
    stitch::{ModuleStitch, ModuleStitchInfo},
};

pub struct Table {
    pub table_type: TableType,
    pub init: TableInit,
}
pub struct TableType {
    pub initial: u32,
    pub maximum: Option<u32>,
}
pub enum TableInit {
    RefNull,
    Expr(PreparedInstructionList),
}
impl Table {
    pub fn new(table: wasmparser::Table) -> anyhow::Result<Self> {
        Ok(Self {
            table_type: TableType::new(table.ty)?,
            init: TableInit::new(table.init)?,
        })
    }

    pub fn stitch(&self, module: &mut ModuleStitch, offsets: &ModuleStitchInfo) {
        match self.init {
            TableInit::RefNull => module.tables.table(self.table_type.encode()),
            TableInit::Expr(ref expr) => module
                .tables
                .table_with_init(self.table_type.encode(), &expr.encode(offsets)),
        };
    }
}
impl TableType {
    pub fn new(ty: wasmparser::TableType) -> anyhow::Result<Self> {
        if ty.table64 || ty.shared || ty.element_type != wasmparser::RefType::FUNCREF {
            // this should be covered by the validator at the start
            panic!("invalid ty passed to TableType::new: {ty:?}");
        }
        Ok(Self {
            initial: ty.initial as u32,
            maximum: ty.maximum.map(|m| m as u32),
        })
    }
    fn encode(&self) -> wasm_encoder::TableType {
        wasm_encoder::TableType {
            element_type: wasm_encoder::RefType::FUNCREF,
            table64: false,
            minimum: self.initial as u64,
            maximum: self.maximum.map(|m| m as u64),
            shared: false,
        }
    }
}
impl TableInit {
    pub fn new(table: wasmparser::TableInit) -> anyhow::Result<Self> {
        Ok(match table {
            wasmparser::TableInit::RefNull => Self::RefNull,
            wasmparser::TableInit::Expr(expr) => Self::Expr(expr.try_into()?),
        })
    }
}

pub struct ElementSegment {
    pub kind: ElementKind,
    // always function indices. references aren't allowed
    pub items: Box<[u32]>,
}
/// See [`wasm_encoder::ElementMode`].
pub enum ElementKind {
    Passive,
    Active {
        table_index: u32,
        offset: PreparedInstructionList,
    },
}
impl ElementSegment {
    pub fn new(elem: wasmparser::Element) -> anyhow::Result<Self> {
        let wasmparser::ElementItems::Functions(funcs) = elem.items else {
            anyhow::bail!("reftype element passed to ElementSegment::new");
        };
        Ok(Self {
            kind: ElementKind::new(elem.kind)?,
            items: funcs.into_iter().collect::<Result<Box<[u32]>, _>>()?,
        })
    }
}
impl ElementKind {
    pub fn new(kind: wasmparser::ElementKind) -> anyhow::Result<Self> {
        Ok(match kind {
            wasmparser::ElementKind::Passive => Self::Passive,
            wasmparser::ElementKind::Active {
                table_index,
                offset_expr,
            } => Self::Active {
                table_index: table_index.unwrap_or(0),
                offset: offset_expr.try_into()?,
            },
            // Declared elements only occur with function types
            wasmparser::ElementKind::Declared => {
                panic!("declared element segments are not allowed")
            }
        })
    }
}

pub struct DataSegment {
    pub mode: DataSegmentMode,
    pub data: Box<[u8]>,
}
pub enum DataSegmentMode {
    Active {
        offset: PreparedInstructionList,
        memory_index: u32,
    },
    Passive,
}

pub struct Global {
    pub ty: GlobalType,
    pub init: PreparedInstructionList,
}
impl Global {
    pub fn new(global: wasmparser::Global) -> anyhow::Result<Self> {
        Ok(Self {
            ty: GlobalType::new(global.ty)?,
            init: global.init_expr.try_into()?,
        })
    }
}
pub struct GlobalType {
    pub val_type: wasm_encoder::ValType,
    pub mutable: bool,
}
impl GlobalType {
    pub fn new(t: wasmparser::GlobalType) -> anyhow::Result<Self> {
        assert!(
            !matches!(t.content_type, wasmparser::ValType::Ref(_)),
            "ref types are not supported :("
        );
        Ok(Self {
            // if the type isn't a ref type, val_type() never returns Err(_)
            val_type: wasm_encoder::reencode::RoundtripReencoder
                .val_type(t.content_type)
                .expect("unreachable"),
            mutable: t.mutable,
        })
    }

    pub fn encode(&self) -> wasm_encoder::GlobalType {
        wasm_encoder::GlobalType {
            val_type: self.val_type,
            mutable: self.mutable,
            shared: false,
        }
    }
}
