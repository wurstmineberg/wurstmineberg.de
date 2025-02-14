use {
    std::{
        collections::HashMap,
        env,
        fs::{
            self,
            File,
        },
        io::prelude::*,
        path::{
            Path,
            PathBuf,
        },
    },
    gix::{
        ObjectId,
        Repository,
    },
    itertools::Itertools as _,
};

fn check_static_file(cache: &mut HashMap<PathBuf, ObjectId>, repo: &Repository, relative_path: &Path, path: PathBuf) -> Result<(), Error> {
    let mut iter_commit = repo.head_commit()?;
    let commit_id = loop {
        let iter_commit_id = iter_commit.id();
        let parents = iter_commit.parent_ids().collect_vec();
        let [parent] = &*parents else {
            // initial commit or merge commit; mark the file as updated here for simplicity's sake
            break iter_commit_id
        };
        let parent = parent.object()?.peel_to_commit()?;
        let diff = repo.diff_tree_to_tree(&parent.tree()?, &iter_commit.tree()?, None)?;
        if path.to_str().is_none_or(|path| diff.into_iter().any(|change| change.location() == path)) {
            break iter_commit_id
        }
        iter_commit = parent;
    };
    cache.insert(relative_path.to_owned(), commit_id.into());
    Ok(())
}

fn check_static_dir(cache: &mut HashMap<PathBuf, ObjectId>, repo: &Repository, relative_path: &Path, path: PathBuf) -> Result<(), Error> {
    for entry in fs::read_dir(&path)? {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            check_static_dir(cache, repo, &relative_path.join(entry.file_name()), entry.path())?;
        } else {
            check_static_file(cache, repo, &relative_path.join(entry.file_name()), entry.path())?;
        }
    }
    Ok(())
}

#[derive(Debug, thiserror::Error)]
enum Error {
    #[error(transparent)] GitCommit(#[from] gix::object::commit::Error),
    #[error(transparent)] GitDiff(#[from] gix::repository::diff_tree_to_tree::Error),
    #[error(transparent)] GitFind(#[from] gix::object::find::existing::Error),
    #[error(transparent)] GitHeadCommit(#[from] gix::reference::head_commit::Error),
    #[error(transparent)] GitOpen(#[from] gix::open::Error),
    #[error(transparent)] GitPeel(#[from] gix::object::peel::to_kind::Error),
    #[error(transparent)] Io(#[from] std::io::Error),
}

fn main() -> Result<(), Error> {
    println!("cargo:rerun-if-changed=nonexistent.foo"); // check a nonexistent file to make sure build script is always run (see https://github.com/rust-lang/cargo/issues/4213 and https://github.com/rust-lang/cargo/issues/5663)
    let static_dir = Path::new("assets").join("static");
    let mut cache = HashMap::default();
    let repo = gix::open(&env::var_os("CARGO_MANIFEST_DIR").unwrap())?;
    for entry in fs::read_dir(&static_dir)? {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            check_static_dir(&mut cache, &repo, entry.file_name().as_ref(), entry.path())?;
        } else {
            check_static_file(&mut cache, &repo, entry.file_name().as_ref(), entry.path())?;
        }
    }
    let mut out_f = File::create(Path::new(&env::var_os("OUT_DIR").unwrap()).join("build_output.rs"))?;
    writeln!(&mut out_f, "macro_rules! static_url {{")?;
    for (path, commit_id) in cache {
        let unix_path = path.to_str().expect("non-UTF-8 static file path").replace('\\', "/");
        let uri = format!("/static/{unix_path}?v={commit_id}");
        writeln!(&mut out_f, "    ({unix_path:?}) => {{")?;
        writeln!(&mut out_f, "        ::rocket_util::Origin(::rocket::uri!({uri:?}))")?;
        writeln!(&mut out_f, "    }};")?;
    }
    writeln!(&mut out_f, "}}")?;
    writeln!(&mut out_f, "const YEAR_OF_LAST_COMMIT: i32 = {};", repo.head_commit()?.time()?.format(gix::diff::object::date::time::CustomFormat::new("%Y")))?;
    Ok(())
}
