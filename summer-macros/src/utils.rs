use syn::{ItemFn, ReturnType, FnArg, Type};

/// 从函数定义中提取所有参数类型和返回类型，过滤掉 impl Trait
pub fn extract_fn_types(ast: &ItemFn) -> (Vec<&Type>, Option<&Type>) {
    let mut input_tys = Vec::new();

    // 提取参数类型，跳过 impl Trait
    for arg in &ast.sig.inputs {
        if let FnArg::Typed(pat_type) = arg {
            let ty = &*pat_type.ty;
            if !matches!(ty, Type::ImplTrait(_)) {
                input_tys.push(ty);
            }
        }
    }

    // 提取返回类型，跳过 impl Trait
    let output_ty = match &ast.sig.output {
        ReturnType::Type(_, ty) if !matches!(&**ty, Type::ImplTrait(_)) => Some(&**ty),
        _ => None,
    };

    (input_tys, output_ty)
}
