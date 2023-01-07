mod utils;
mod wasm_lang;

use wasm_lang::WASMLang;

use ast_grep_config::{
  deserialize_rule, try_deserialize_matchers, RuleWithConstraint, SerializableMetaVarMatcher,
  SerializableRule,
};
use ast_grep_core::meta_var::MetaVarMatchers;
use ast_grep_core::{Node, Pattern};
use std::collections::HashMap;
use utils::WASMMatch;
use ast_grep_core::language::Language;

use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;


#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

#[derive(Serialize, Deserialize)]
pub struct WASMConfig {
  pub rule: SerializableRule,
  pub fix: Option<String>,
  pub constraints: Option<HashMap<String, SerializableMetaVarMatcher>>,
}

#[wasm_bindgen(js_name = setupParser)]
pub async fn setup_parser(lang_name: String, parser_path: String) -> Result<(), JsError> {
  WASMLang::set_current(&lang_name, &parser_path).await
}

#[wasm_bindgen(js_name = findNodes)]
pub fn find_nodes(src: String, config: JsValue) -> Result<JsValue, JsError> {
  let config: WASMConfig = serde_wasm_bindgen::from_value(config)?;
  let lang = WASMLang::get_current();
  let root = lang.ast_grep(src);
  let rule = deserialize_rule(config.rule, lang.clone())?;
  let matchers = if let Some(c) = config.constraints {
    try_deserialize_matchers(c, lang).unwrap()
  } else {
    MetaVarMatchers::default()
  };
  let config = RuleWithConstraint { rule, matchers };
  let ret: Vec<_> = root.root().find_all(config).map(WASMMatch::from).collect();
  let ret = serde_wasm_bindgen::to_value(&ret)?;
  Ok(ret)
}

#[wasm_bindgen(js_name = fixErrors)]
pub fn fix_errors(src: String, config: JsValue) -> Result<String, JsError> {
  let config: WASMConfig = serde_wasm_bindgen::from_value(config)?;
  let lang = WASMLang::get_current();
  let fixer = config.fix.expect_throw("fix is required for rewriting");
  let fixer = Pattern::new(&fixer, lang.clone());
  let root = lang.ast_grep(&src);
  let rule = deserialize_rule(config.rule, lang.clone())?;
  let matchers = if let Some(c) = config.constraints {
    try_deserialize_matchers(c, lang).unwrap()
  } else {
    MetaVarMatchers::default()
  };
  let config = RuleWithConstraint { rule, matchers };
  let edits: Vec<_> = root.root().replace_all(config, fixer);
  let mut new_content = String::new();
  let mut start = 0;
  for edit in edits {
    new_content.push_str(&src[start..edit.position]);
    new_content.push_str(&edit.inserted_text);
    start = edit.position + edit.deleted_length;
  }
  // add trailing statements
  new_content.push_str(&src[start..]);
  Ok(new_content)
}

#[derive(Deserialize, Serialize)]
struct DebugNode {
  kind: String,
  start: (usize, usize),
  end: (usize, usize),
  is_named: bool,
  children: Vec<DebugNode>,
}

fn convert_to_debug_node(n: Node<WASMLang>) -> DebugNode {
  let children = n.children().map(convert_to_debug_node).collect();
  DebugNode {
    kind: n.kind().to_string(),
    start: n.start_pos(),
    end: n.end_pos(),
    is_named: n.is_named(),
    children,
  }
}

#[wasm_bindgen(js_name = dumpASTNodes)]
pub fn dump_ast_nodes(src: String) -> Result<JsValue, JsError> {
  let lang = WASMLang::get_current();
  let root = lang.ast_grep(&src);
  let debug_node = convert_to_debug_node(root.root());
  let ret = serde_wasm_bindgen::to_value(&debug_node)?;
  Ok(ret)
}
