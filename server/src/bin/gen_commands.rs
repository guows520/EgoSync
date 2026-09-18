//! gen_commands —— Story 15.4 commands.json 工件机械导出器（OQ2 裁决 A）。
//!
//! 机制：syn 源码扫描 `crates/egosync-engine/src/commands/*.rs`，对每条
//! 「web-ok 命令」导出 名字 + 参数 schema（ctx 注入与客户端参数标注区分）
//! + capability；desktop-only 12 条与 perf-test 门控 2 条经 engine
//! capabilities 名单物理排除。产物两件：
//! 1. `crates/egosync-engine/commands.json` —— 构建期工件（CI 新鲜度断言）；
//! 2. `server/src/dispatch_gen.rs` —— 生成的 dispatch registry（路由层入册）。
//!
//! 命令判定规则（纯类型结构、零人肉清单）：
//! `pub async fn` 且每个参数均为按值客户端参数或单个 `ctx: &EngineCtx`——
//! 持有其他引用参数（`&DbPool` / `&Arc<...>` / `&dyn ...`）者为内部辅助
//! 函数（如 suggestion 的 confirm_and_create_task），非命令。
//!
//! 运行：`cd server && cargo run --features gen --bin gen_commands`。

use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::PathBuf;

use syn::{Item, ItemFn, UseTree, Visibility};

/// 工程根目录（server/ 的上级）。
fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..")
}

/// 引擎命令目录。
fn commands_dir() -> PathBuf {
    repo_root().join("crates/egosync-engine/src/commands")
}

/// 一条命令的参数描述。
#[derive(Debug, Clone)]
struct Param {
    /// Rust 参数名（snake_case）。
    name: String,
    /// camelCase 客户端名（与 invoke 同构）。
    camel_name: String,
    /// 类型（客户端参数为解析后的全限定类型；ctx 为 `&EngineCtx`）。
    ty: String,
    /// ctx 注入参数 or 客户端参数。
    kind: ParamKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ParamKind {
    Ctx,
    Client,
}

/// 一条命令的完整描述。
#[derive(Debug, Clone)]
struct Command {
    name: String,
    /// 所在引擎命令模块（文件名去扩展）。
    module: String,
    params: Vec<Param>,
}

fn main() {
    let dir = commands_dir();
    let mut commands: Vec<Command> = Vec::new();
    let mut skipped: Vec<String> = Vec::new();

    let mut entries: Vec<PathBuf> = std::fs::read_dir(&dir)
        .expect("读取 engine commands 目录失败")
        .map(|e| e.expect("目录项读取失败").path())
        .filter(|p| p.extension().is_some_and(|e| e == "rs"))
        .collect();
    entries.sort();

    for path in entries {
        let module = path.file_stem().unwrap().to_string_lossy().to_string();
        // mod.rs（模块声明）与 ctx.rs（EngineCtx 容器）非命令域文件
        if module == "mod" || module == "ctx" {
            continue;
        }
        let src = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("读取 {} 失败: {}", path.display(), e));
        let file = syn::parse_file(&src)
            .unwrap_or_else(|e| panic!("解析 {} 失败: {}", path.display(), e));
        let use_map = build_use_map(&file);
        let local_types = collect_local_type_names(&file);

        for item in &file.items {
            let Item::Fn(ItemFn { vis, sig, .. }) = item else {
                continue;
            };
            if !matches!(vis, Visibility::Public(_)) || sig.asyncness.is_none() {
                continue; // 非公开异步函数（私有/pub(crate) 辅助）非命令候选
            }
            let fn_name = sig.ident.to_string();
            // 名单排除：desktop-only / perf-test 门控物理不导出
            if egosync_engine::capabilities::is_desktop_only(&fn_name)
                || egosync_engine::capabilities::is_perf_test_gated(&fn_name)
            {
                skipped.push(fn_name);
                continue;
            }
            assert!(
                sig.generics.params.is_empty(),
                "命令 {} 不应携带泛型参数（ctx 化后宿主泛型已退役）",
                fn_name
            );

            let mut params = Vec::new();
            let mut is_command = true;
            for arg in &sig.inputs {
                let syn::FnArg::Typed(pat_type) = arg else {
                    panic!("命令 {} 不应有 self 参数", fn_name);
                };
                let name = match pat_type.pat.as_ref() {
                    syn::Pat::Ident(ident) => ident.ident.to_string(),
                    _ => panic!("命令 {} 参数名非简单标识符", fn_name),
                };
                match pat_type.ty.as_ref() {
                    syn::Type::Reference(ty_ref) => {
                        // 引用参数：仅 `ctx: &EngineCtx` 为命令合法形态；其余
                        //（&DbPool / &Arc<...> / &dyn ...）判为内部辅助函数
                        if is_plain_engine_ctx(&ty_ref.elem) {
                            let camel_name = name.clone();
                            params.push(Param {
                                name,
                                camel_name,
                                ty: "&EngineCtx".to_string(),
                                kind: ParamKind::Ctx,
                            });
                        } else {
                            is_command = false;
                            break;
                        }
                    }
                    syn::Type::Path(_) => {
                        let rendered = render_type(
                            pat_type.ty.as_ref(),
                            module.as_str(),
                            &use_map,
                            &local_types,
                        )
                        .unwrap_or_else(|| panic!("命令 {} 参数类型无法机械解析", fn_name));
                        let camel_name = to_camel_case(&name);
                        params.push(Param {
                            name,
                            camel_name,
                            ty: rendered,
                            kind: ParamKind::Client,
                        });
                    }
                    _ => {
                        // trait object / 元组等复杂形态非客户端参数
                        is_command = false;
                        break;
                    }
                }
            }
            if is_command {
                commands.push(Command {
                    name: fn_name,
                    module: module.clone(),
                    params,
                });
            }
        }
    }

    commands.sort_by(|a, b| a.name.cmp(&b.name));
    let names: HashSet<&str> = commands.iter().map(|c| c.name.as_str()).collect();
    assert_eq!(names.len(), commands.len(), "命令名重复（不同模块出现同名命令）");
    println!(
        "导出 web-ok 命令 {} 条（排除 desktop-only/perf-test 门控 {} 处）",
        commands.len(),
        skipped.len()
    );

    write_commands_json(&commands);
    write_dispatch_gen(&commands);
}

/// 是否为裸 `EngineCtx` 路径（ctx 注入参数的判别形态）。
fn is_plain_engine_ctx(ty: &syn::Type) -> bool {
    let syn::Type::Path(type_path) = ty else {
        return false;
    };
    type_path.qself.is_none()
        && type_path.path.leading_colon.is_none()
        && type_path.path.segments.len() == 1
        && type_path.path.segments[0].ident == "EngineCtx"
        && type_path.path.segments[0].arguments.is_none()
}

/// 构建 use 树映射：类型名 → 全限定路径（`crate::` 前缀映射为
/// `egosync_engine::`）。
fn build_use_map(file: &syn::File) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for item in &file.items {
        let Item::Use(use_item) = item else { continue };
        walk_use_tree(&use_item.tree, "", &mut map);
    }
    map
}

/// 递归展开 use 树（支持任意深度嵌套分组）。
fn walk_use_tree(tree: &UseTree, prefix: &str, map: &mut HashMap<String, String>) {
    match tree {
        UseTree::Path(path) => {
            let next = if prefix.is_empty() {
                path.ident.to_string()
            } else {
                format!("{}::{}", prefix, path.ident)
            };
            walk_use_tree(&path.tree, &next, map);
        }
        UseTree::Name(name) => {
            let full = normalize_use_path(&format!("{}::{}", prefix, name.ident));
            map.insert(name.ident.to_string(), full);
        }
        UseTree::Rename(rename) => {
            let full = normalize_use_path(&format!("{}::{}", prefix, rename.ident));
            map.insert(rename.rename.to_string(), full);
        }
        UseTree::Group(group) => {
            for inner in &group.items {
                walk_use_tree(inner, prefix, map);
            }
        }
        UseTree::Glob(_) => {
            // 通配导入不展开（引擎命令文件无 glob use；出现即机械性失效）
            panic!("engine 命令文件不应使用 glob use（无法机械解析类型归属）");
        }
    }
}

/// `crate::` 前缀映射为 `egosync_engine::`；其余（std/外部 crate）原样。
fn normalize_use_path(path: &str) -> String {
    if let Some(rest) = path.strip_prefix("crate::") {
        format!("egosync_engine::{}", rest)
    } else {
        path.to_string()
    }
}

/// 收集文件内本地声明的公开类型名（struct/enum/type alias）。
fn collect_local_type_names(file: &syn::File) -> HashSet<String> {
    let mut names = HashSet::new();
    for item in &file.items {
        match item {
            Item::Struct(s) => {
                names.insert(s.ident.to_string());
            }
            Item::Enum(e) => {
                names.insert(e.ident.to_string());
            }
            Item::Type(t) => {
                names.insert(t.ident.to_string());
            }
            _ => {}
        }
    }
    names
}

/// 渲染类型为字符串：use-map 命中的标识符替换为全限定路径；本地类型加
/// 模块前缀；其余（std prelude / 原始类型）原样。
/// 返回 None 表示该类型形态不可作为客户端参数（trait object 等）。
fn render_type(
    ty: &syn::Type,
    module: &str,
    use_map: &HashMap<String, String>,
    local_types: &HashSet<String>,
) -> Option<String> {
    let syn::Type::Path(type_path) = ty else {
        return None;
    };
    let mut segments = Vec::new();
    for (i, seg) in type_path.path.segments.iter().enumerate() {
        let ident = seg.ident.to_string();
        let rendered = if i == 0
            && type_path.path.leading_colon.is_none()
            && type_path.qself.is_none()
        {
            if let Some(full) = use_map.get(&ident) {
                full.clone()
            } else if local_types.contains(&ident) {
                format!("egosync_engine::commands::{}::{}", module, ident)
            } else {
                // std prelude（String/Vec/Option/原始类型）
                ident
            }
        } else {
            ident
        };
        match &seg.arguments {
            syn::PathArguments::None => segments.push(rendered),
            syn::PathArguments::AngleBracketed(bracketed) => {
                let inner: Vec<String> = bracketed
                    .args
                    .iter()
                    .map(|arg| match arg {
                        syn::GenericArgument::Type(inner_ty) => {
                            render_type(inner_ty, module, use_map, local_types)
                                .unwrap_or_default()
                        }
                        other => panic!("不支持的泛型参数形态"),
                    })
                    .collect();
                segments.push(format!("{}<{}>", rendered, inner.join(", ")));
            }
            other => panic!("不支持的路径参数形态"),
        }
    }
    Some(segments.join("::"))
}

/// snake_case → camelCase（与 invoke 参数键同构）。
fn to_camel_case(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    let mut upper_next = false;
    for ch in name.chars() {
        if ch == '_' {
            upper_next = true;
        } else if upper_next {
            out.extend(ch.to_uppercase());
            upper_next = false;
        } else {
            out.push(ch);
        }
    }
    out
}

/// snake_case → PascalCase（生成参数结构体名）。
fn to_pascal_case(name: &str) -> String {
    let camel = to_camel_case(name);
    let mut chars = camel.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

/// 写出 commands.json（确定性排序、无时间戳——CI 新鲜度零漂移）。
fn write_commands_json(commands: &[Command]) {
    #[derive(serde::Serialize)]
    struct ParamJson {
        name: String,
        #[serde(rename = "camelName")]
        camel_name: String,
        r#type: String,
        kind: &'static str,
    }
    #[derive(serde::Serialize)]
    struct CommandJson {
        name: String,
        module: String,
        capability: &'static str,
        #[serde(rename = "ctxInjected")]
        ctx_injected: bool,
        params: Vec<ParamJson>,
    }

    let body: Vec<CommandJson> = commands
        .iter()
        .map(|c| CommandJson {
            name: c.name.clone(),
            module: c.module.clone(),
            capability: "web-ok",
            ctx_injected: c.params.iter().any(|p| p.kind == ParamKind::Ctx),
            params: c
                .params
                .iter()
                .map(|p| ParamJson {
                    name: p.name.clone(),
                    camel_name: p.camel_name.clone(),
                    r#type: p.ty.clone(),
                    kind: match p.kind {
                        ParamKind::Ctx => "ctx-injected",
                        ParamKind::Client => "client",
                    },
                })
                .collect(),
        })
        .collect();

    let mut ordered = serde_json::Map::new();
    ordered.insert("version".to_string(), serde_json::json!(1));
    ordered.insert("count".to_string(), serde_json::json!(body.len()));
    ordered.insert(
        "commands".to_string(),
        serde_json::to_value(&body).expect("命令清单序列化失败"),
    );
    let doc = serde_json::Value::Object(ordered);

    let path = repo_root().join("crates/egosync-engine/commands.json");
    let text = serde_json::to_string_pretty(&doc).expect("commands.json 序列化失败");
    std::fs::write(&path, text + "\n").expect("写 commands.json 失败");
    println!("已写出 {}", path.display());
}

/// 生成 server/src/dispatch_gen.rs（dispatch registry 代码镜像）。
fn write_dispatch_gen(commands: &[Command]) {
    let mut arms = String::new();
    let mut consts = String::new();
    let mut used_structs: BTreeMap<String, String> = BTreeMap::new();

    for cmd in commands {
        let client_params: Vec<&Param> = cmd
            .params
            .iter()
            .filter(|p| p.kind == ParamKind::Client)
            .collect();

        let call_args = cmd
            .params
            .iter()
            .map(|p| match p.kind {
                ParamKind::Ctx => "ctx".to_string(),
                ParamKind::Client => format!("p.{}", p.name),
            })
            .collect::<Vec<_>>()
            .join(",\n            ");

        let decode = if client_params.is_empty() {
            String::new()
        } else {
            let struct_name = format!("P{}", to_pascal_case(&cmd.name));
            let mut struct_def = format!(
                "    /// `{}` 的客户端参数（camelCase 解包，与 invoke 同构）。\n    #[derive(serde::Deserialize)]\n    #[serde(rename_all = \"camelCase\")]\n    pub struct {} {{\n",
                cmd.name, struct_name
            );
            for p in &client_params {
                let default_attr = if p.ty.starts_with("Option<") {
                    "        #[serde(default)]\n"
                } else {
                    ""
                };
                struct_def.push_str(&format!(
                    "{}        pub {}: {},\n",
                    default_attr, p.name, p.ty
                ));
            }
            struct_def.push_str("    }\n\n");
            used_structs.insert(struct_name.clone(), struct_def);

            format!("        let p: params::{} = decode(params)?;\n", struct_name)
        };

        let call = format!(
            "        let result = egosync_engine::commands::{}::{}(\n            {}\n        ).await?;\n        encode(result)\n",
            cmd.module, cmd.name, call_args
        );

        arms.push_str(&format!(
            "        \"{}\" => {{\n{}{}        }}\n",
            cmd.name, decode, call
        ));
        consts.push_str(&format!("    \"{}\",\n", cmd.name));
    }

    let mut params_defs = String::new();
    for def in used_structs.values() {
        params_defs.push_str(def);
    }

    let code = format!(
        r#"//! AUTO-GENERATED by `cargo run --features gen --bin gen_commands` —— 请勿手改。
//!
//! Story 15.4 工件代码镜像：web-ok 命令 dispatch registry（与
//! `crates/egosync-engine/commands.json` 同源机械导出）。
//! 参数 camelCase 反序列化与桌面 invoke 同构（Option 缺省 → None）；
//! 业务错误一律 `AppError`（路由层 200 + 单键 map）；未知命令 → NotFound。

use egosync_engine::commands::ctx::EngineCtx;
use egosync_engine::error::AppError;

/// web-ok 命令全名单（生成物；对等断言与 commands.json 工件互验）。
pub const WEB_OK_COMMANDS: &[&str] = &[
{consts}];

mod params {{
{params_defs}}}

/// 解码客户端参数（camelCase body → 命令参数；与 invoke 同构）。
/// 类型不符 ⇒ `ValidationError`（路由层 200 单键 map 形状）。
fn decode<T: serde::de::DeserializeOwned>(params: serde_json::Value) -> Result<T, AppError> {{
    serde_json::from_value(params)
        .map_err(|e| AppError::ValidationError(format!("参数反序列化失败: {{}}", e)))
}}

/// 编码命令返回值。
fn encode<T: serde::Serialize>(value: T) -> Result<serde_json::Value, AppError> {{
    serde_json::to_value(value)
        .map_err(|e| AppError::ValidationError(format!("返回值序列化失败: {{}}", e)))
}}

/// 命令分发：按名路由至引擎命令体（首参 ctx 注入）。
/// 未知名（含 desktop-only / perf-test 门控）→ `NotFound`（路由层 404）。
pub async fn dispatch(
    ctx: &EngineCtx,
    command: &str,
    params: serde_json::Value,
) -> Result<serde_json::Value, AppError> {{
    match command {{
{arms}        _ => Err(AppError::NotFound(format!("未知命令: {{}}", command))),
    }}
}}
"#,
        consts = consts,
        params_defs = params_defs,
        arms = arms,
    );

    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/dispatch_gen.rs");
    std::fs::write(&path, code).expect("写 dispatch_gen.rs 失败");
    println!("已写出 {}", path.display());
}
