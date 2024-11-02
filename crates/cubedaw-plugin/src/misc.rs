use wasm_encoder::reencode::Reencode;

use crate::{
    prepare::{PrepareContext, PreparedInstructionList},
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
    pub fn new(ctx: &PrepareContext, table: wasmparser::Table) -> anyhow::Result<Self> {
        Ok(Self {
            table_type: TableType::new(table.ty)?,
            init: TableInit::new(ctx, table.init)?,
        })
    }

    pub fn stitch(&self, module: &mut ModuleStitch, info: &ModuleStitchInfo) {
        match self.init {
            TableInit::RefNull => module.tables.table(self.table_type.encode()),
            TableInit::Expr(ref expr) => module
                .tables
                .table_with_init(self.table_type.encode(), &expr.encode(info)),
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
    pub fn new(ctx: &PrepareContext, table: wasmparser::TableInit) -> anyhow::Result<Self> {
        Ok(match table {
            wasmparser::TableInit::RefNull => Self::RefNull,
            wasmparser::TableInit::Expr(expr) => {
                Self::Expr(PreparedInstructionList::from_constexpr(ctx, &expr)?)
            }
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
    pub fn new(ctx: &PrepareContext, elem: wasmparser::Element) -> anyhow::Result<Self> {
        let wasmparser::ElementItems::Functions(funcs) = elem.items else {
            anyhow::bail!("reftype element passed to ElementSegment::new");
        };
        Ok(Self {
            kind: ElementKind::new(ctx, elem.kind)?,
            items: funcs.into_iter().collect::<Result<Box<[u32]>, _>>()?,
        })
    }
}
impl ElementKind {
    pub fn new(ctx: &PrepareContext, kind: wasmparser::ElementKind) -> anyhow::Result<Self> {
        Ok(match kind {
            wasmparser::ElementKind::Passive => Self::Passive,
            wasmparser::ElementKind::Active {
                table_index,
                offset_expr,
            } => Self::Active {
                table_index: table_index.unwrap_or(0),
                offset: PreparedInstructionList::from_constexpr(ctx, &offset_expr)?,
            },
            // Declared elements use a WASM feature that plugins don't have access to
            // so this is unreachable
            wasmparser::ElementKind::Declared => {
                panic!("declared element segments are not allowed")
            }
        })
    }
}

pub struct DataSegment {
    pub mode: DataSegmentKind,
    pub data: Box<[u8]>,
}
pub enum DataSegmentKind {
    Active {
        offset: PreparedInstructionList,
        memory_index: u32,
    },
    Passive,
}
impl DataSegment {
    pub fn new(ctx: &PrepareContext, data: wasmparser::Data) -> anyhow::Result<Self> {
        Ok(Self {
            mode: DataSegmentKind::new(ctx, data.kind)?,
            data: data.data.into(),
        })
    }
}
impl DataSegmentKind {
    pub fn new(ctx: &PrepareContext, kind: wasmparser::DataKind) -> anyhow::Result<Self> {
        Ok(match kind {
            wasmparser::DataKind::Active {
                memory_index,
                offset_expr,
            } => Self::Active {
                offset: PreparedInstructionList::from_constexpr(ctx, &offset_expr)?,
                memory_index,
            },
            wasmparser::DataKind::Passive => Self::Passive,
        })
    }
}

#[derive(Debug)]
pub struct Global {
    pub ty: GlobalType,
    pub init: PreparedInstructionList,
}
impl Global {
    pub fn new(ctx: &PrepareContext, global: wasmparser::Global) -> anyhow::Result<Self> {
        Ok(Self {
            ty: GlobalType::new(global.ty)?,
            init: PreparedInstructionList::from_constexpr(ctx, &global.init_expr)?,
        })
    }
}
#[derive(Debug)]
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
