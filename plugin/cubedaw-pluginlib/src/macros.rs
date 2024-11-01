#[macro_export]
macro_rules! declare_plugin {
    (@1
        id: $value:literal
        $($args:tt)*
    ) => {
        const _: &str = $value;
        $crate::__postcard_stringify::declare! {
            #[link_section = "cubedaw:plugin_meta"]
            static _CUBEDAWPLUGIN_ID = "id", $value;
        }

        $crate::declare_plugin!(@2 $($args)*);
    };
    (@1
        name: $value:literal
        $($args:tt)*
    ) => {
        const _: &str = $value;
        $crate::__postcard_stringify::declare! {
            #[link_section = "cubedaw:plugin_meta"]
            static _CUBEDAWPLUGIN_NAME = "name", $value;
        }

        $crate::declare_plugin!(@2 $($args)*);
    };
    (@1
        description: $value:literal
        $($args:tt)*
    ) => {
        const _: &str = $value;
        $crate::__postcard_stringify::declare! {
            #[link_section = "cubedaw:plugin_meta"]
            static _CUBEDAWPLUGIN_DESCRIPTION = "description", $value;
        }

        $crate::declare_plugin!(@2 $($args)*);
    };


    (@1
        $key:ident: $val:tt
        $($args:tt)*
    ) => {
        compile_error!(concat!("invalid key \"", stringify!($key), "\" in plugin declaration"));
    };
    (@1) => {};
    (@2, $($args:tt)*) => {
        $crate::declare_plugin!(@1 $($args)*);
    };
    (@2) => {};
    (@1 $($args:tt)*) => {
        compile_error!(concat!("invalid syntax in plugin declaration: ", stringify!($($args)+)));
    };

    // TODO: more entries; license, version, etc etc
    ($($args:tt)*) => {
        #[link_section = "cubedaw:plugin_version"]
        static _CUBEDAWPLUGIN_VERSION: [u8; 5] = *b"0.1.0";

        $crate::declare_plugin!(@1 $($args)*);
    };
    ($($args:tt)*) => {
        compile_error!("invalid syntax in plugin declaration");
    };
}

// TODO: this would be better as an attribute proc macro
#[macro_export]
macro_rules! export_node {
    ($name:literal, $function:ident) => {
        const _: &str = $name;

        $crate::__paste::paste! {
            $crate::__postcard_stringify::declare! {
                #[link_section = "cubedaw:node_list"]
                static [<_CUBEDAWPLUGIN_ $function:upper>] = $name, stringify!($function);
            }
        }
    };
}

pub use paste as __paste;
pub use postcard_stringify as __postcard_stringify;
