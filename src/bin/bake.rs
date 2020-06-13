// wengwengweng

pub fn bake_syntax() -> Result<(), String> {

	use syntect::parsing::SyntaxSetBuilder;
	use syntect::parsing::syntax_definition::SyntaxDefinition;

	macro_rules! syntax {
		($name:expr) => {
			SyntaxDefinition::load_from_str(
				include_str!(concat!("../bufs/syntaxes/", $name, ".sublime-syntax")),
				true,
				Some($name),
			).unwrap()
		}
	}

	let mut ssb = SyntaxSetBuilder::new();

	ssb.add(syntax!("Rust"));
	ssb.add(syntax!("GLSL"));
	ssb.add(syntax!("TOML"));
	ssb.add(syntax!("Markdown"));
	ssb.add(syntax!("Lua"));
	ssb.add(syntax!("JavaScript"));
	ssb.add(syntax!("Makefile"));
	ssb.add(syntax!("C"));
	ssb.add(syntax!("C++"));
	ssb.add(syntax!("CSS"));
	ssb.add(syntax!("HTML"));
	ssb.add(syntax!("JSON"));
	ssb.add(syntax!("YAML"));
	ssb.add(syntax!("Python"));
	ssb.add(syntax!("Ruby"));
	ssb.add(syntax!("Lisp"));
	ssb.add(syntax!("Go"));
	ssb.add_plain_text_syntax();

	let syntax_set = ssb.build();

	return syntect::dumps::dump_to_file(
		&syntax_set,
		concat!(env!("CARGO_MANIFEST_DIR"), "/src/bufs/syntaxset.pack")
	)
		.map_err(|_| format!("failed to dump pack"));

}

fn main() {
	if let Err(e) = bake_syntax() {
		eprintln!("{}", e);
	}
}

