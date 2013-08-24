use std::{borrow, io, option};

use syntax::abi;
use syntax::ast;
use syntax::codemap::{dummy_sp, dummy_spanned, ExpnInfo, NameAndSpan};
use syntax::ast_util::*;
use syntax::ext::base;
use syntax::ext::build::AstBuilder;
use syntax::parse;
use syntax::print::pprust;
use syntax::opt_vec;

use types::*;

struct GenCtx {
    ext_cx: @base::ExtCtxt,
    unnamed_ty: uint,
    abis: abi::AbiSet
}

fn empty_generics() -> ast::Generics {
    ast::Generics {
        lifetimes: opt_vec::Empty,
        ty_params: opt_vec::Empty
    }
}

fn rust_id(ctx: &mut GenCtx, name: ~str) -> (~str, bool) {
    let token = parse::token::IDENT(ctx.ext_cx.ident_of(name), false);
    if parse::token::is_any_keyword(&token) || "bool" == name {
        (~"_" + name, true)
    } else {
        (name, false)
    }

}

fn rust_type_id(ctx: &mut GenCtx, name: ~str) -> ~str {
    if "bool" == name ||
        "uint" == name ||
        "u8" == name ||
        "u16" == name ||
        "u32" == name ||
        "f32" == name ||
        "f64" == name ||
        "i8" == name ||
        "i16" == name ||
        "i32" == name ||
        "i64" == name ||
        "Self" == name ||
        "str" == name {
        ~"_" + name
    } else {
        let (n, _) = rust_id(ctx, name);
        n
    }
}

fn unnamed_name(ctx: &mut GenCtx, name: ~str) -> ~str {
    return if name.is_empty() {
        ctx.unnamed_ty += 1;
        fmt!("Unnamed%u", ctx.unnamed_ty)
    } else {
        name
    };
}

fn struct_name(name: ~str) -> ~str {
    fmt!("Struct_%s", name)
}

fn union_name(name: ~str) -> ~str {
    fmt!("Union_%s", name)
}

fn enum_name(name: ~str) -> ~str {
    fmt!("Enum_%s", name)
}

pub fn gen_rs(out: @io::Writer, abi: ~str, link: &Option<~str>, globs: &[Global]) {
    let abis = match abi {
        ~"cdecl" => abi::AbiSet::from(abi::Cdecl),
        ~"stdcall" => abi::AbiSet::from(abi::Stdcall),
        ~"fastcall" => abi::AbiSet::from(abi::Fastcall),
        ~"aapcs" => abi::AbiSet::from(abi::Aapcs),
        ~"Rust" => abi::AbiSet::Rust(),
        ~"rust-intrinsic" => abi::AbiSet::Intrinsic(),
        _ => abi::AbiSet::C()
    };

    let mut ctx = GenCtx { ext_cx: base::ExtCtxt::new(parse::new_parse_sess(None), ~[]),
                           unnamed_ty: 0,
                           abis: abis
                         };
    ctx.ext_cx.bt_push(ExpnInfo {
        call_site: dummy_sp(),
        callee: NameAndSpan { name: @"", span: None }
    });
    let uniq_globs = tag_dup_decl(globs);

    let mut fs = ~[];
    let mut vs = ~[];
    let mut gs = ~[];
    uniq_globs.iter().advance(|g| {
        match *g {
            GOther => {}
            GFunc(_) => fs.push(*g),
            GVar(_) => vs.push(*g),
            _ => gs.push(*g)
        }
        true
    });

    let mut defs = ~[];
    gs = remove_redundent_decl(gs);

    gs.iter().advance(|g| {
        match *g {
            GType(ti) => defs.push_all(ctypedef_to_rs(&mut ctx, ti.name.clone(), ti.ty)),
            GCompDecl(ci) => {
                ci.name = unnamed_name(&mut ctx, ci.name.clone());
                if ci.cstruct {
                    defs.push_all(ctypedef_to_rs(&mut ctx, struct_name(ci.name.clone()), @TVoid))
                } else {
                    defs.push_all(ctypedef_to_rs(&mut ctx, union_name(ci.name.clone()), @TVoid))
                }
            },
            GComp(ci) => {
                ci.name = unnamed_name(&mut ctx, ci.name.clone());
                if ci.cstruct {
                    defs.push(cstruct_to_rs(&mut ctx, struct_name(ci.name.clone()),
                                            ci.fields.clone()))
                } else {
                    defs.push_all(cunion_to_rs(&mut ctx, union_name(ci.name.clone()),
                                               ci.fields.clone()))
                }
            },
            GEnumDecl(ei) => {
                ei.name = unnamed_name(&mut ctx, ei.name.clone());
                defs.push_all(ctypedef_to_rs(&mut ctx, enum_name(ei.name.clone()), @TVoid))
            },
            GEnum(ei) => {
                ei.name = unnamed_name(&mut ctx, ei.name.clone());
                defs.push_all(cenum_to_rs(&mut ctx, enum_name(ei.name.clone()), ei.items.clone(),
                                          ei.kind))
            },
            _ => { }
        }
        true
    });

    let vars = do vs.map |v| {
        match *v {
            GVar(vi) => cvar_to_rs(&mut ctx, vi.name.clone(), vi.ty),
            _ => { fail!(~"generate global variables") }
        }
    };

    let funcs = do fs.map |f| {
        match *f {
            GFunc(vi) => {
                match *vi.ty {
                    TFunc(rty, ref aty, var) => cfunc_to_rs(&mut ctx, vi.name.clone(),
                                                             rty, (*aty).clone(), var),
                    _ => { fail!(~"generate functions") }
                }
            },
            _ => { fail!(~"generate functions") }
        }
    };

    let views = ~[mk_import(&mut ctx, &[~"std", ~"libc"])];
    defs.push(mk_extern(&mut ctx, link, vars, funcs));

    let crate = ast::Crate {
        module: ast::_mod {
            view_items: views,
            items: defs,
        },
        attrs: ~[],
        config: ~[],
        span: dummy_sp()
    };

    let ps = pprust::rust_printer(out, parse::token::get_ident_interner());
    out.write_line("/* automatically generated by rust-bindgen */\n");
    pprust::print_crate_(ps, &crate);
}

fn mk_import(ctx: &mut GenCtx, path: &[~str]) -> ast::view_item {
    let view = ast::view_item_use(~[
        @dummy_spanned(
            ast::view_path_glob(
                ast::Path {
                   span: dummy_sp(),
                   global: false,
                   idents: path.map(|p| ctx.ext_cx.ident_of((*p).clone())),
                   rp: None,
                   types: ~[]
                },
                ctx.ext_cx.next_id()
            )
        )
    ]);

    return ast::view_item {
              node: view,
              attrs: ~[],
              vis: ast::inherited,
              span: dummy_sp()
           };
}

fn mk_extern(ctx: &mut GenCtx, link: &Option<~str>,
                           vars: ~[@ast::foreign_item],
                           funcs: ~[@ast::foreign_item]) -> @ast::item {
    let attrs;
    match *link {
        None => attrs = ~[],
        Some(ref l) => {
            let link_args = dummy_spanned(ast::Attribute_ {
                style: ast::AttrOuter,
                value: @dummy_spanned(
                    ast::MetaNameValue(
                        @"link_args",
                        dummy_spanned(ast::lit_str((~"-l" + *l).to_managed()))
                    )
                ),
                is_sugared_doc: false
            });
            attrs = ~[link_args];
        }
    }

    let ext = ast::item_foreign_mod(ast::foreign_mod {
        sort: ast::anonymous,
        abis: ctx.abis,
        view_items: ~[],
        items: vars + funcs
    });

    return @ast::item {
              ident: ctx.ext_cx.ident_of(""),
              attrs: attrs,
              id: ctx.ext_cx.next_id(),
              node: ext,
              vis: ast::public,
              span: dummy_sp()
           };
}

fn remove_redundent_decl(gs: &[Global]) -> ~[Global] {
    fn check_decl(a: Global, b: Global) -> bool {
        match (a, b) {
          (GComp(ci1), GType(ti)) => match *ti.ty {
              TComp(ci2) => {
                  let n = ci1.name.clone();
                  borrow::ref_eq(ci1, ci2) && n.is_empty()
              },
              _ => false
          },
          (GEnum(ei1), GType(ti)) => match *ti.ty {
              TEnum(ei2) => {
                  let n = ei1.name.clone();
                  borrow::ref_eq(ei1, ei2) && n.is_empty()
              },
              _ => false
          },
          _ => false
        }
    }

    let gsit = gs.iter();
    let typedefs: ~[Global] = gsit.filter_map(|g|
        match(*g) {
            GType(_) => Some(*g),
            _ => None
        }
    ).collect();

    return gsit.filter_map(|g|
        if typedefs.iter().any(|t| check_decl(*g, *t)) {
            None
        } else {
            Some(*g)
        }
    ).collect();
}

fn tag_dup_decl(gs: &[Global]) -> ~[Global] {
    fn check(g1: Global, g2: Global) -> Global {
        if !g1.to_str().is_empty() && g1.to_str() == g2.to_str() {
            GOther
        } else {
            g2
        }
    }

    fn check_dup(g1: Global, g2: Global) -> Global {
        match (g1, g2) {
          (GType(_), GType(_)) => check(g1, g2),
          (GComp(_), GComp(_)) => check(g1, g2),
          (GCompDecl(_), GCompDecl(_)) => check(g1, g2),
          (GEnum(_), GEnum(_)) => check(g1, g2),
          (GEnumDecl(_), GEnumDecl(_)) => check(g1, g2),
          (GVar(_), GVar(_)) => check(g1, g2),
          (GFunc(_), GFunc(_)) => check(g1, g2),
          _ => g2
        }
    }

    let mut res = gs.map(|g| *g);
    let len = res.len();
    let mut i = 0;

    while i < len {
        let mut j = i + 1;

        while j < len {
            let g2 = check_dup(res[i], res[j]);
            res[j] = g2;
            j += 1;
        }
        i += 1;
    }
    return res;
}

fn ctypedef_to_rs(ctx: &mut GenCtx, name: ~str, ty: @Type) -> ~[@ast::item] {
    fn mk_item(ctx: &mut GenCtx, name: ~str, ty: @Type) -> @ast::item {
        let rust_name = rust_type_id(ctx, name);
        let rust_ty = cty_to_rs(ctx, ty);
        let base = ast::item_ty(
            ast::Ty {
                id: ctx.ext_cx.next_id(),
                node: rust_ty.node.clone(),
                span: dummy_sp(),
            },
            empty_generics()
        );

        return @ast::item {
                  ident: ctx.ext_cx.ident_of(rust_name),
                  attrs: ~[],
                  id: ctx.ext_cx.next_id(),
                  node: base,
                  vis: ast::public,
                  span: dummy_sp()
               };
    }

    return match *ty {
        TComp(ci) => {
            let n = ci.name.clone();
            if n.is_empty() {
                ci.name = name.clone();
                if ci.cstruct {
                    ~[cstruct_to_rs(ctx, name, ci.fields.clone())]
                } else {
                    cunion_to_rs(ctx, name, ci.fields.clone())
                }
            } else {
                ~[mk_item(ctx, name, ty)]
            }
        },
        TEnum(ei) => {
            let n = ei.name.clone();
            if n.is_empty() {
                ei.name = name.clone();
                cenum_to_rs(ctx, name, ei.items.clone(), ei.kind)
            } else {
                ~[mk_item(ctx, name, ty)]
            }
        },
        _ => ~[mk_item(ctx, name, ty)]
    }
}

fn cstruct_to_rs(ctx: &mut GenCtx, name: ~str, fields: ~[@FieldInfo]) -> @ast::item {
    let mut unnamed = 0;
    let fs = do fields.map |f| {
        let n = f.name.clone();
        let f_name = if n.is_empty() {
            unnamed += 1;
            fmt!("unnamed_field%u", unnamed)
        } else {
            rust_type_id(ctx, f.name.clone())
        };

        let f_ty = cty_to_rs(ctx, f.ty);

        @dummy_spanned(ast::struct_field_ {
            kind: ast::named_field(
                ctx.ext_cx.ident_of(f_name),
                ast::inherited
            ),
            id: ctx.ext_cx.next_id(),
            ty: f_ty,
            attrs: ~[]
        })
    };

    let def = ast::item_struct(
        @ast::struct_def {
           fields: fs,
           ctor_id: None
        },
        empty_generics()
    );

    let id = rust_type_id(ctx, name);
    return @ast::item { ident: ctx.ext_cx.ident_of(id),
              attrs: ~[],
              id: ctx.ext_cx.next_id(),
              node: def,
              vis: ast::public,
              span: dummy_sp()
           };
}

fn cunion_to_rs(ctx: &mut GenCtx, name: ~str, fields: ~[@FieldInfo]) -> ~[@ast::item] {
    fn mk_item(ctx: &mut GenCtx, name: ~str, item: ast::item_) -> @ast::item {
        return @ast::item {
                  ident: ctx.ext_cx.ident_of(name),
                  attrs: ~[],
                  id: ctx.ext_cx.next_id(),
                  node: item,
                  vis: ast::public,
                  span: dummy_sp()
               };
    }

    let ext_cx = ctx.ext_cx;
    let ci = mk_compinfo(name.clone(), false, ~[]);
    ci.fields = fields.clone();
    let union = @TNamed(mk_typeinfo(name.clone(), @TComp(ci)));

    let data = @dummy_spanned(ast::struct_field_ {
        kind: ast::named_field(
            ext_cx.ident_of("data"),
            ast::inherited
        ),
        id: ext_cx.next_id(),
        ty: cty_to_rs(ctx, @TArray(@TInt(IUChar), type_size(union))),
        attrs: ~[]
    });

    let def = ast::item_struct(
        @ast::struct_def {
           fields: ~[data],
           ctor_id: None
        },
        empty_generics()
    );
    let union_id = rust_type_id(ctx, name);
    let union_def = mk_item(ctx, union_id, def);

    let expr = quote_expr!(
        ext_cx,
        unsafe { std::cast::transmute(&std::ptr::to_mut_unsafe_ptr(self)) }
    );
    let mut unnamed = 0;
    let fs = do fields.map |f| {
        let n = f.name.clone();
        let f_name = if n.is_empty() {
            unnamed += 1;
            fmt!("unnamed_field%u", unnamed)
        } else {
            rust_id(ctx, f.name.clone()).first()
        };

        let ret_ty = cty_to_rs(ctx, @TPtr(f.ty, false));
        let body = ast::Block {
            view_items: ~[],
            stmts: ~[],
            expr: Some(expr),
            id: ext_cx.next_id(),
            rules: ast::DefaultBlock,
            span: dummy_sp()
        };

        @ast::method {
            ident: ext_cx.ident_of(f_name),
            attrs: ~[],
            generics: empty_generics(),
            explicit_self: dummy_spanned(ast::sty_region(None, ast::m_mutbl)),
            purity: ast::impure_fn,
            decl: ast::fn_decl {
                inputs: ~[],
                output: ret_ty,
                cf: ast::return_val
            },
            body: body,
            id: ext_cx.next_id(),
            span: dummy_sp(),
            self_id: union_def.id,
            vis: ast::public
        }
    };

    let methods = ast::item_impl(
        empty_generics(),
        None,
        cty_to_rs(ctx, union),
        fs
    );

    return ~[
        union_def,
        mk_item(ctx, ~"", methods)
    ];
}

fn cenum_to_rs(ctx: &mut GenCtx, name: ~str, items: ~[@EnumItem], kind: IKind) -> ~[@ast::item] {
    let ty = @TInt(kind);
    let ty_id = rust_type_id(ctx, name);
    let ty_def = ctypedef_to_rs(ctx, ty_id, ty);
    let val_ty = cty_to_rs(ctx, ty);
    let mut def = ty_def;

    items.iter().advance(|it| {
        let cst = ast::item_static(
            val_ty.clone(),
            ast::m_imm,
            ctx.ext_cx.expr_int(dummy_sp(), it.val)
        );

        let id = rust_id(ctx, it.name.clone()).first();
        let val_def = @ast::item {
                         ident: ctx.ext_cx.ident_of(id),
                         attrs: ~[],
                         id: ctx.ext_cx.next_id(),
                         node: cst,
                         vis: ast::public,
                         span: dummy_sp()
                      };

        def.push(val_def);
        true
    });

    return def;
}

fn mk_link_name_attr(name: ~str) -> ast::Attribute {
    let lit = dummy_spanned(ast::lit_str(name.to_managed()));
    let attr_val = @dummy_spanned(ast::MetaNameValue(@"link_name", lit));
    let attr = ast::Attribute_ {
        style: ast::AttrOuter,
        value: attr_val,
        is_sugared_doc: false
    };
    dummy_spanned(attr)
}

fn cvar_to_rs(ctx: &mut GenCtx, name: ~str, ty: @Type) -> @ast::foreign_item {
    let (rust_name, was_mangled) = rust_id(ctx, name.clone());

    let mut attrs = ~[];
    if was_mangled {
        attrs.push(mk_link_name_attr(name));
    }

    return @ast::foreign_item {
              ident: ctx.ext_cx.ident_of(rust_name),
              attrs: attrs,
              node: ast::foreign_item_static(cty_to_rs(ctx, ty), false),
              id: ctx.ext_cx.next_id(),
              span: dummy_sp(),
              vis: ast::public,
           };
}

fn cfuncty_to_rs(ctx: &mut GenCtx, rty: @Type,
                                    aty: ~[(~str, @Type)],
                                    _var: bool) -> ast::fn_decl {

    let ret = match *rty {
        TVoid => ast::Ty {
            id: ctx.ext_cx.next_id(),
            node: ast::ty_nil,
            span: dummy_sp()
        },
        _ => cty_to_rs(ctx, rty)
    };

    let mut unnamed = 0;
    let args = do aty.map |arg| {
        let (n, t) = (*arg).clone();

        let arg_name = if n.is_empty() {
            unnamed += 1;
            fmt!("arg%u", unnamed)
        } else {
            rust_id(ctx, n).first()
        };

        let arg_ty = cty_to_rs(ctx, t);

        ast::arg {
            is_mutbl: false,
            ty: arg_ty,
            pat: @ast::pat {
                 id: ctx.ext_cx.next_id(),
                 node: ast::pat_ident(
                     ast::bind_infer,
                     ast::Path {
                         span: dummy_sp(),
                         global: false,
                         idents: ~[ctx.ext_cx.ident_of(arg_name)],
                         rp: None,
                         types: ~[]
                     },
                     None
                 ),
                 span: dummy_sp()
            },
            id: ctx.ext_cx.next_id()
        }
    };

    return ast::fn_decl {
        inputs: args,
        output: ret,
        cf: ast::return_val
    };
}

fn cfunc_to_rs(ctx: &mut GenCtx, name: ~str, rty: @Type,
                                         aty: ~[(~str, @Type)],
                                         var: bool) -> @ast::foreign_item {
    let decl = ast::foreign_item_fn(
        cfuncty_to_rs(ctx, rty, aty, var),
        empty_generics()
    );

    let (rust_name, was_mangled) = rust_id(ctx, name.clone());

    let mut attrs = ~[];
    if was_mangled {
        attrs.push(mk_link_name_attr(name));
    }

    return @ast::foreign_item {
              ident: ctx.ext_cx.ident_of(rust_name),
              attrs: attrs,
              node: decl,
              id: ctx.ext_cx.next_id(),
              span: dummy_sp(),
              vis: ast::public,
           };
}

fn cty_to_rs(ctx: &mut GenCtx, ty: @Type) -> ast::Ty {
    return match *ty {
        TVoid => mk_ty(ctx, ~"c_void"),
        TInt(i) => match i {
            IBool => mk_ty(ctx, ~"c_int"),
            ISChar => mk_ty(ctx, ~"c_schar"),
            IUChar => mk_ty(ctx, ~"c_uchar"),
            IInt => mk_ty(ctx, ~"c_int"),
            IUInt => mk_ty(ctx, ~"c_uint"),
            IShort => mk_ty(ctx, ~"c_short"),
            IUShort => mk_ty(ctx, ~"c_ushort"),
            ILong => mk_ty(ctx, ~"c_long"),
            IULong => mk_ty(ctx, ~"c_ulong"),
            ILongLong => mk_ty(ctx, ~"c_longlong"),
            IULongLong => mk_ty(ctx, ~"c_ulonglong")
        },
        TFloat(f) => match f {
            FFloat => mk_ty(ctx, ~"c_float"),
            FDouble => mk_ty(ctx, ~"c_double")
        },
        TPtr(t, is_const) => {
            let id = cty_to_rs(ctx, t);
            mk_ptrty(ctx, &id, is_const)
        },
        TArray(t, s) => {
            let ty = cty_to_rs(ctx, t);
            mk_arrty(ctx, &ty, s)
        },
        TFunc(rty, ref atys, var) => {
            let decl = cfuncty_to_rs(ctx, rty, (*atys).clone(), var);
            mk_fnty(ctx, &decl)
        },
        TNamed(ti) => {
            let id = rust_type_id(ctx, ti.name.clone());
            mk_ty(ctx, id)
        },
        TComp(ci) => {
            ci.name = unnamed_name(ctx, ci.name.clone());
            if ci.cstruct {
                mk_ty(ctx, struct_name(ci.name.clone()))
            } else {
                mk_ty(ctx, union_name(ci.name.clone()))
            }
        },
        TEnum(ei) => {
            ei.name = unnamed_name(ctx, ei.name.clone());
            mk_ty(ctx, enum_name(ei.name.clone()))
        }
    };
}

fn mk_ty(ctx: &mut GenCtx, name: ~str) -> ast::Ty {
    let ty = ast::ty_path(
        ast::Path {
            span: dummy_sp(),
            global: false,
            idents: ~[ctx.ext_cx.ident_of(name)],
            rp: None,
            types: ~[]
        },
        option::None,
        ctx.ext_cx.next_id()
    );

    return ast::Ty {
        id: ctx.ext_cx.next_id(),
        node: ty,
        span: dummy_sp()
    };
}

fn mk_ptrty(ctx: &mut GenCtx, base: &ast::Ty, is_const: bool) -> ast::Ty {
    let ty = ast::ty_ptr(ast::mt{
        ty: ~(*base).clone(),
        mutbl: if is_const { ast::m_imm } else { ast::m_mutbl }
    });

    return ast::Ty {
        id: ctx.ext_cx.next_id(),
        node: ty,
        span: dummy_sp()
    };
}

fn mk_arrty(ctx: &mut GenCtx, base: &ast::Ty, n: uint) -> ast::Ty {
    let sz = ast::expr_lit(@dummy_spanned(ast::lit_uint(n as u64, ast::ty_u)));
    let ty = ast::ty_fixed_length_vec(
        ast::mt {
            ty: ~(*base).clone(),
            mutbl: ast::m_imm
        },
        @ast::expr {
            id: ctx.ext_cx.next_id(),
            node: sz,
            span: dummy_sp()
        }
    );

    return ast::Ty {
        id: ctx.ext_cx.next_id(),
        node: ty,
        span: dummy_sp()
    };
}

fn mk_fnty(ctx: &mut GenCtx, decl: &ast::fn_decl) -> ast::Ty {
    let fnty = ast::ty_bare_fn(@ast::TyBareFn {
        purity: ast::impure_fn,
        abis: ctx.abis,
        lifetimes: opt_vec::Empty,
        decl: (*decl).clone()
    });

    return ast::Ty {
        id: ctx.ext_cx.next_id(),
        node: fnty,
        span: dummy_sp()
    };
}
