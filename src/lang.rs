use {
    std::fmt,
    itertools::Itertools as _,
    nonempty_collections::{
        IntoIteratorExt as _,
        IntoNonEmptyIterator,
        NEVec,
        NonEmptyIterator as _,
    },
};

fn join<T: fmt::Display>(elts: impl IntoNonEmptyIterator<Item = T>) -> String {
    join_with("and", elts)
}

pub(crate) fn join_opt<T: fmt::Display>(elts: impl IntoIterator<Item = T>) -> Option<String> {
    elts.try_into_nonempty_iter().map(|iter| join(iter))
}

fn join_with<T: fmt::Display>(conjunction: &str, elts: impl IntoNonEmptyIterator<Item = T>) -> String {
    let (first, rest) = elts.into_nonempty_iter().next();
    let mut rest = rest.fuse();
    match (rest.next(), rest.next()) {
        (None, _) => first.to_string(),
        (Some(second), None) => format!("{first} {conjunction} {second}"),
        (Some(second), Some(third)) => {
            let mut rest = [second, third].into_nonempty_iter().chain(rest).collect::<NEVec<_>>();
            let last = rest.pop().expect("rest contains at least second and third");
            format!("{first}, {}, {conjunction} {last}", rest.into_iter().format(", "))
        }
    }
}
