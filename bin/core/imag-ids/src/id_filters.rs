//
// imag - the personal information management suite for the commandline
// Copyright (C) 2015-2018 Matthias Beyer <mail@beyermatthias.de> and contributors
//
// This library is free software; you can redistribute it and/or
// modify it under the terms of the GNU Lesser General Public
// License as published by the Free Software Foundation; version
// 2.1 of the License.
//
// This library is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU
// Lesser General Public License for more details.
//
// You should have received a copy of the GNU Lesser General Public
// License along with this library; if not, write to the Free Software
// Foundation, Inc., 51 Franklin Street, Fifth Floor, Boston, MA  02110-1301  USA
//

use filters::filter::Filter;

use libimagstore::storeid::StoreId;

pub struct IsInCollectionsFilter<'a, A>(Option<A>, ::std::marker::PhantomData<&'a str>)
    where A: AsRef<[&'a str]>;

impl<'a, A> IsInCollectionsFilter<'a, A>
    where A: AsRef<[&'a str]>
{
    pub fn new(collections: Option<A>) -> Self {
        IsInCollectionsFilter(collections, ::std::marker::PhantomData)
    }
}

impl<'a, A> Filter<StoreId> for IsInCollectionsFilter<'a, A>
    where A: AsRef<[&'a str]> + 'a
{
    fn filter(&self, sid: &StoreId) -> bool {
        match self.0 {
            Some(ref colls) => sid.is_in_collection(colls),
            None => true,
        }
    }
}

/// Language definition for the header-filter language
///
/// # Notes
///
/// Here are some notes how the language should look like:
///
/// ```ignore
/// query = filter (operator filter)*
///
/// filter = unary? ((function "(" selector ")" ) | selector ) compare_op compare_val
///
/// unary = "not"
///
/// compare_op =
///     "is"     |
///     "in"     |
///     "==/eq"  |
///     "!=/neq" |
///     ">="     |
///     "<="     |
///     "<"      |
///     ">"      |
///     "any"    |
///     "all"
///
/// compare_val = val | listofval
///
/// val         = string | int | float | bool
/// listofval   = "[" (val ",")* "]"
///
/// operator =
///     "or"      |
///     "or_not"  |
///     "and"     |
///     "and_not" |
///     "xor"
///
/// function =
///     "length" |
///     "keys"   |
///     "values"
/// ```
///
pub mod header_filter_lang {
    use std::str;
    use std::str::FromStr;
    use std::process::exit;

    use nom::digit;

    use libimagstore::store::Entry;
    use libimagerror::trace::MapErrTrace;

    #[derive(Debug, PartialEq, Eq)]
    enum Unary {
        Not
    }

    named!(unary_operator<Unary>, alt_complete!(
        tag!("not") => { |_| Unary::Not }
    ));

    #[derive(Debug, PartialEq, Eq)]
    enum CompareOp {
        OpIs,
        OpIn,
        OpEq,
        OpNeq,
        OpGte, // >=
        OpLte, // <=
        OpLt,  // <
        OpGt,  // >
    }

    named!(compare_op<CompareOp>, alt_complete!(
        tag!("is"     ) => { |_| CompareOp::OpIs }  |
        tag!("in"     ) => { |_| CompareOp::OpIn }  |
        tag!("==/eq"  ) => { |_| CompareOp::OpEq }  |
        tag!("!=/neq" ) => { |_| CompareOp::OpNeq } |
        tag!(">="     ) => { |_| CompareOp::OpGte } |
        tag!("<="     ) => { |_| CompareOp::OpLte } |
        tag!("<"      ) => { |_| CompareOp::OpLt }  |
        tag!(">"      ) => { |_| CompareOp::OpGt }
    ));

    #[derive(Debug, PartialEq, Eq)]
    enum Operator {
        Or,
        And,
        Xor,
    }

    named!(operator<Operator>, alt_complete!(
        tag!("or")      => { |_| Operator::Or }     |
        tag!("and")     => { |_| Operator::And }    |
        tag!("xor")     => { |_| Operator::Xor }
    ));

    #[derive(Debug, PartialEq, Eq)]
    enum Function {
        Length,
        Keys,
        Values,
    }

    named!(function<Function>, alt_complete!(
        tag!("length") => { |_| Function::Length } |
        tag!("keys")   => { |_| Function::Keys }   |
        tag!("values") => { |_| Function::Values }
    ));

    #[derive(Debug, PartialEq, Eq)]
    enum Value {
        Integer(i64),
        String(String),
    }

    named!(int64<i64>, map!(digit, |r: &[u8]| {
        let val = str::from_utf8(r).unwrap_or_else(|e| {
            error!("Error = '{:?}'", e);
            ::std::process::exit(1)
        });

        i64::from_str(val).unwrap_or_else(|e| {
            error!("Error while parsing number: '{:?}'", e);
            ::std::process::exit(1)
        })
    }));

    named!(signed_digits<(Option<&[u8]>, i64)>,
        pair!(opt!(alt!(tag_s!("+") | tag_s!("-"))), int64)
    );
    named!(integer<i64>, do_parse!(tpl: signed_digits >> (match tpl.0 {
        Some(b"-") => -tpl.1,
        _          => tpl.1,
    })));

    named!(string<String>, do_parse!(
       text: delimited!(char!('"'), is_not!("\""), char!('"'))
       >> (String::from_utf8(text.to_vec()).unwrap())
    ));

    named!(val<Value>, alt_complete!(
        do_parse!(number: integer >> (Value::Integer(number))) |
        do_parse!(text: string >> (Value::String(text)))
    ));

    named!(list_of_val<Vec<Value>>, do_parse!(
            char!('[') >> list: many0!(terminated!(val, opt!(char!(',')))) >> char!(']') >> (list)
    ));

    #[derive(Debug, PartialEq, Eq)]
    enum CompareValue {
        Value(Value),
        Values(Vec<Value>)
    }

    named!(compare_value<CompareValue>, alt_complete!(
        do_parse!(list: list_of_val >> (CompareValue::Values(list))) |
        do_parse!(val: val >> (CompareValue::Value(val)))
    ));

    #[derive(Debug, PartialEq, Eq)]
    enum Selector {
        Direct(String),
        Function(Function, String)
    }

    impl Selector {
        fn selector_str(&self) -> &String {
            match *self {
                Selector::Direct(ref s)      => s,
                Selector::Function(_, ref s) => s,
            }
        }
        fn function(&self) -> Option<&Function> {
            match *self {
                Selector::Direct(_)          => None,
                Selector::Function(ref f, _) => Some(f),
            }
        }
    }

    named!(selector_str<String>, do_parse!(
        selector: take_till!(|s: u8| s.is_ascii_whitespace()) >> (String::from_utf8(selector.to_vec()).unwrap())
    ));

    named!(selector<Selector>, alt_complete!(
        do_parse!(fun: function >> sel: delimited!(char!('('), selector_str, char!(')')) >> (Selector::Function(fun, sel))) |
        do_parse!(sel: selector_str >> (Selector::Direct(sel)))
    ));

    #[derive(Debug, PartialEq, Eq)]
    struct Filter {
        unary            : Option<Unary>,
        selector         : Selector,
        compare_operator : CompareOp,
        compare_value    : CompareValue,
    }

    named!(filter<Filter>, do_parse!(
            unary: opt!(unary_operator) >>
            selec: selector >>
            comop: compare_op >>
            cmval: compare_value >>
            (Filter {
                unary:              unary,
                selector:           selec,
                compare_operator:   comop,
                compare_value:      cmval,
            })
    ));

    #[derive(Debug, PartialEq, Eq)]
    pub struct Query {
        filter: Filter,
        next_filters: Vec<(Operator, Filter)>,
    }

    named!(parse_query<Query>, do_parse!(
            filt: filter >>
            next: many0!(do_parse!(op: operator >> fil: filter >> ((op, fil)))) >>
            (Query {
                filter:       filt,
                next_filters: next,
            })
    ));

    /// Helper type which can filters::filter::Filter be implemented on so that the implementation
    /// of ::filters::filter::Filter on self::Filter is less complex.
    struct Comparator<'a>(&'a CompareOp, &'a CompareValue);

    impl<'a> ::filters::filter::Filter<::toml::Value> for Comparator<'a> {
        fn filter(&self, val: &::toml::Value) -> bool {
            use self::CompareValue as CV;
            use self::CompareOp    as CO;
            use toml::Value        as TVal;

            match *self.0 {
                CO::OpIs => match self.1 {
                    &CV::Values(_) => error_exit("Cannot check whether a header field is the same type as mulitple values!"),
                    &CV::Value(ref v) => match v {
                        &Value::Integer(_) => is_match!(*val, TVal::Integer(_)),
                        &Value::String(_)  => is_match!(val, &TVal::String(_)),
                    },
                },
                CO::OpIn => match (self.1, val) {
                    (&CV::Value(Value::Integer(i)), &TVal::Integer(j))       => i == j,
                    (&CV::Value(Value::String(ref s)), &TVal::String(ref b)) => s.contains(b),
                    (&CV::Value(_), _)                                       => false,

                    (&CV::Values(ref v), &TVal::Integer(j)) => v.iter().any(|e| match e {
                        &Value::Integer(i) => i == j,
                        _                  => false
                    }),
                    (&CV::Values(ref v), &TVal::String(ref b)) => v.iter().any(|e| match e {
                        &Value::String(ref s) => s == b,
                        _                     => false
                    }),
                    (&CV::Values(_), _) => false,
                },
                CO::OpEq => match (self.1, val) {
                    (&CV::Value(Value::Integer(i)), &TVal::Integer(j))       => i == j,
                    (&CV::Value(Value::String(ref s)), &TVal::String(ref b)) => s == b,
                    (&CV::Value(_), _)  => false,
                    (&CV::Values(_), _) => error_exit("Cannot check a header field for equality to multiple header fields!"),
                },
                CO::OpNeq => match (self.1, val) {
                    (&CV::Value(Value::Integer(i)), &TVal::Integer(j)) => i != j,
                    (&CV::Value(_), _)  => false,
                    (&CV::Values(_), _) => error_exit("Cannot check a header field for inequality to multiple header fields!"),
                },
                CO::OpGte => match (self.1, val) {
                    (&CV::Value(Value::Integer(i)), &TVal::Integer(j)) => i >= j,
                    (&CV::Value(_), _)  => false,
                    (&CV::Values(_), _) => error_exit("Cannot check a header field for greater_than_equal to multiple header fields!"),
                },
                CO::OpLte => match (self.1, val) {
                    (&CV::Value(Value::Integer(i)), &TVal::Integer(j)) => i <= j,
                    (&CV::Value(_), _)  => false,
                    (&CV::Values(_), _) => error_exit("Cannot check a header field for lesser_than_equal to multiple header fields!"),
                },
                CO::OpLt => match (self.1, val) {
                    (&CV::Value(Value::Integer(i)), &TVal::Integer(j)) => i < j,
                    (&CV::Value(_), _)  => false,
                    (&CV::Values(_), _) => error_exit("Cannot check a header field for lesser_than to multiple header fields!"),
                },
                CO::OpGt => match (self.1, val) {
                    (&CV::Value(Value::Integer(i)), &TVal::Integer(j)) => i > j,
                    (&CV::Value(_), _)  => false,
                    (&CV::Values(_), _) => {
                        error!("Cannot check a header field for greater_than to multiple header fields!");
                        exit(1)
                    },
                },
            }
        }
    }

    impl ::filters::filter::Filter<Entry> for Filter {
        fn filter(&self, entry: &Entry) -> bool {
            use toml_query::read::TomlValueReadExt;

            entry
                .get_header()
                .read(self.selector.selector_str())
                .map_err_trace_exit_unwrap(1)
                .map(|value| {
                    let comp = Comparator(&self.compare_operator, &self.compare_value);
                    let val = match self.selector.function() {
                        None => {
                            ::filters::filter::Filter::filter(&comp, value)
                        }
                        Some(func) => {
                            match *func {
                                Function::Length => {
                                    let val = match value {
                                        &::toml::Value::Array(ref a)  => a.len() as i64,
                                        &::toml::Value::String(ref s) => s.len() as i64,
                                        _                            => 1
                                    };
                                    let val = ::toml::Value::Integer(val);
                                    ::filters::filter::Filter::filter(&comp, &val)
                                },
                                Function::Keys => {
                                    let keys = match value {
                                        &::toml::Value::Table(ref tab) => tab
                                            .keys()
                                            .cloned()
                                            .map(::toml::Value::String)
                                            .collect(),
                                        _ => return false,
                                    };
                                    let keys = ::toml::Value::Array(keys);
                                    ::filters::filter::Filter::filter(&comp, &keys)
                                },
                                Function::Values => {
                                    let vals = match value {
                                        &::toml::Value::Table(ref tab) => tab
                                            .values()
                                            .cloned()
                                            .collect(),
                                        _ => return false,
                                    };
                                    let vals = ::toml::Value::Array(vals);
                                    ::filters::filter::Filter::filter(&comp, &vals)
                                },
                            }
                        }
                    };

                    match self.unary {
                        Some(Unary::Not) => !val,
                        _                => val
                    }
                })
                .unwrap_or(false)
        }
    }

    impl ::filters::filter::Filter<Entry> for Query {

        fn filter(&self, entry: &Entry) -> bool {
            let mut res = self.filter.filter(entry);

            for &(ref operator, ref next) in self.next_filters.iter() {
                match *operator {
                    Operator::Or => {
                        res = res || ::filters::filter::Filter::filter(next, entry);
                    },
                    Operator::And => {
                        res = res && ::filters::filter::Filter::filter(next, entry);
                    },
                    Operator::Xor => {
                        let other = ::filters::filter::Filter::filter(next, entry);
                        res = (res && !other) || (!res && other);
                    },
                }
            }

            res
        }

    }

    fn error_exit(s: &'static str) -> ! {
        error!("{}", s);
        exit(1)
    }

    pub fn parse(s: &str) -> Query {
        match parse_query(s.as_bytes()) {
            ::nom::IResult::Done(_i, o) => o,
            ::nom::IResult::Error(e) => {
                error!("Error during parsing the query");
                error!("Error = {:?}", e);
                ::std::process::exit(1)
            },
            ::nom::IResult::Incomplete(needed) => {
                error!("Error during parsing the query. Incomplete input.");
                error!("Needed = {:?}", needed);
                ::std::process::exit(1)
            },
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn test_unary() {
            assert_eq!(unary_operator(b"not").unwrap().1, Unary::Not);
        }

        #[test]
        fn test_compare_op() {
            assert_eq!(compare_op(b"is"     ).unwrap().1, CompareOp::OpIs );
            assert_eq!(compare_op(b"in"     ).unwrap().1, CompareOp::OpIn );
            assert_eq!(compare_op(b"==/eq"  ).unwrap().1, CompareOp::OpEq );
            assert_eq!(compare_op(b"!=/neq" ).unwrap().1, CompareOp::OpNeq);
            assert_eq!(compare_op(b">="     ).unwrap().1, CompareOp::OpGte);
            assert_eq!(compare_op(b"<="     ).unwrap().1, CompareOp::OpLte);
            assert_eq!(compare_op(b"<"      ).unwrap().1, CompareOp::OpLt );
            assert_eq!(compare_op(b">"      ).unwrap().1, CompareOp::OpGt );
        }

        #[test]
        fn test_operator() {
            assert_eq!(operator(b"or").unwrap().1, Operator::Or  );
            assert_eq!(operator(b"and").unwrap().1, Operator::And );
            assert_eq!(operator(b"xor").unwrap().1, Operator::Xor );
        }

        #[test]
        fn test_function() {
            assert_eq!(function(b"length").unwrap().1, Function::Length );
            assert_eq!(function(b"keys").unwrap().1, Function::Keys );
            assert_eq!(function(b"values").unwrap().1, Function::Values );
        }

        #[test]
        fn test_integer() {
            assert_eq!(integer(b"12").unwrap().1, 12);
            assert_eq!(integer(b"11292").unwrap().1, 11292);
            assert_eq!(integer(b"-12").unwrap().1, -12);
            assert_eq!(integer(b"10101012").unwrap().1, 10101012);
        }

        #[test]
        fn test_string() {
            assert_eq!(string(b"\"foo\"").unwrap().1, "foo");
        }

        #[test]
        fn test_val() {
            assert_eq!(val(b"12").unwrap().1, Value::Integer(12));
            assert_eq!(val(b"\"foobar\"").unwrap().1, Value::String(String::from("foobar")));
        }

        #[test]
        fn test_list_of_val() {
            {
                let list = list_of_val(b"[]");
                println!("list: {:?}", list);
                let vals = list.unwrap().1;
                assert_eq!(vals, vec![]);
            }

            {
                let list = list_of_val(b"[1]");
                println!("list: {:?}", list);
                let vals = list.unwrap().1;
                assert_eq!(vals, vec![Value::Integer(1)]);
            }

            {
                let list = list_of_val(b"[12,13]");
                println!("list: {:?}", list);
                let vals = list.unwrap().1;
                assert_eq!(vals, vec![Value::Integer(12), Value::Integer(13)]);
            }

            {
                let vals = list_of_val(b"[\"foobar\",\"bazbaz\"]").unwrap().1;
                let expt = vec![Value::String(String::from("foobar")),
                                Value::String(String::from("bazbaz"))];
                assert_eq!(vals, expt)
            }
        }

        #[test]
        fn test_selector_str() {
            assert_eq!(selector_str(b"foo.bar baz").unwrap().1, String::from("foo.bar"));
        }

        #[test]
        fn test_selector() {
            assert_eq!(selector(b"foo.bar baz").unwrap().1, Selector::Direct(String::from("foo.bar")));

            let exp = Selector::Function(Function::Length, String::from("foo.bar"));
            assert_eq!(selector(b"length(foo.bar)").unwrap().1, exp);
        }
    }
}

