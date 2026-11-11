//! A package checked on the packages it is stacked on (#1364): what `ofm-cli validate --base` runs,
//! with the one merge the game's stack loader makes.

use std::path::{Path, PathBuf};

use super::package::{
    PackageError, WorldPackage, load_world_package, load_world_package_files,
    load_world_package_from_ofm, merge_world_packages, validate_package,
};

/// A package as `validate` reads it: a directory by its own load, an archive by the runtime read
/// (permissive, so an installed package keeps working) and then the full set of checks, asked for
/// explicitly because validating is the point.
fn read_for_validation(path: &Path) -> (WorldPackage, Vec<PackageError>) {
    if path.is_dir() {
        return load_world_package(path);
    }
    let (package, mut errors) = load_world_package_from_ofm(path);
    errors.extend(validate_package(&package));
    (package, errors)
}

/// A package as the stack reads it: files and ids only, its references left to the merge, whether
/// it is a directory or an archive (the game reads an installed package the same way).
fn read_for_the_stack(path: &Path) -> (WorldPackage, Vec<PackageError>) {
    if path.is_dir() {
        load_world_package_files(path)
    } else {
        load_world_package_from_ofm(path)
    }
}

/// A package (a directory or a `.ofm`) checked on the packages it is stacked on: `bases` (a
/// directory or an archive each), then the package, merged as the game merges a stack it starts a
/// career on.
///
/// Returns the package as read, with its own problems; with no base it stops there, since alone a
/// patch's club references are the stack's to decide (#1271). With bases it adds what the merge
/// reports (the one call `load_world_data_from_package_ids` makes on an installed stack), unless
/// the package already has problems of its own, so none is reported twice. A base that cannot be
/// read, or the package when read again for the merge, reports its read problems instead.
pub fn check_world_package_on_bases(
    path: &Path,
    bases: &[PathBuf],
) -> (WorldPackage, Vec<PackageError>) {
    let (package, errors) = read_for_validation(path);
    if !errors.is_empty() || bases.is_empty() {
        return (package, errors);
    }
    let mut stack = Vec::with_capacity(bases.len() + 1);
    for base in bases {
        let (loaded, base_errors) = read_for_the_stack(base);
        if !base_errors.is_empty() {
            return (package, base_errors);
        }
        stack.push(loaded);
    }
    // The merge consumes what it is given and a package is not `Clone`, so the package is read a
    // second time, as the stack reads it.
    let (for_the_stack, stack_errors) = read_for_the_stack(path);
    if !stack_errors.is_empty() {
        return (package, stack_errors);
    }
    stack.push(for_the_stack);
    let (_merged, findings) = merge_world_packages(stack);
    (package, findings)
}
