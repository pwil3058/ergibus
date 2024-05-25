//Copyright 2019 Peter Williams <pwil3058@gmail.com> <pwil3058@bigpond.net.au>

#[macro_export]
macro_rules! get_row_item_for_event {
    ( $view:ident, $event:ident, $type:ty, $index:expr ) => {{
        let posn = $event.get_position();
        let x = posn.0 as i32;
        let y = posn.1 as i32;
        let mut result: Option<$type> = None;
        if let Some(location) = $view.get_path_at_pos(x, y) {
            if let Some(path) = location.0 {
                if let Some(store) = $view.get_model() {
                    if let Some(iter) = store.get_iter(&path) {
                        result = store.get_value(&iter, $index).get_ok::<$type>()
                    }
                }
            }
        }
        result
    }};
}

#[macro_export]
macro_rules! get_row_item_for_tree_path {
    ( $view:ident, $tree_path:ident, $type:ty, $index:expr ) => {{
        let mut result: Option<$type> = None;
        if let Some(store) = $view.get_model() {
            if let Some(iter) = store.get_iter(&$tree_path) {
                result = store.get_value(&iter, $index).get_ok::<$type>()
            }
        }
        result
    }};
}
