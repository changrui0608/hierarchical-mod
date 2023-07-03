extern crate hierarchical_mod;

hierarchical_mod::path_mod!("./examples");
// hierarchical_mod::auto_mod!();

fn main() {
    foo::foo1::func();
    foo::foo2::func();
    foo::bar::bar::func();
    foo::bar::baz::baz::func();
    foo_bar::foo_bar::func();
    _1_foo::_1::func();
}
