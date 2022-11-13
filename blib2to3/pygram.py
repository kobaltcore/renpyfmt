# Copyright 2006 Google, Inc. All Rights Reserved.
# Licensed to PSF under a Contributor Agreement.

"""Export the Python grammar and symbols."""

# Python imports
import os
import io

from typing import Union

# Local imports
from .pgen2.pgen import ParserGenerator

from .pgen2.grammar import Grammar

# Moved into initialize because mypyc can't handle __file__ (XXX bug)
# # The grammar file
# _GRAMMAR_FILE = os.path.join(os.path.dirname(__file__), "Grammar.txt")
# _PATTERN_GRAMMAR_FILE = os.path.join(os.path.dirname(__file__),
#                                      "PatternGrammar.txt")


class Symbols(object):
    def __init__(self, grammar: Grammar) -> None:
        """Initializer.

        Creates an attribute for each grammar symbol (nonterminal),
        whose value is the symbol's type (an int >= 256).
        """
        for name, symbol in grammar.symbol2number.items():
            setattr(self, name, symbol)


class _python_symbols(Symbols):
    and_expr: int
    and_test: int
    annassign: int
    arglist: int
    argument: int
    arith_expr: int
    asexpr_test: int
    assert_stmt: int
    async_funcdef: int
    async_stmt: int
    atom: int
    augassign: int
    break_stmt: int
    case_block: int
    classdef: int
    comp_for: int
    comp_if: int
    comp_iter: int
    comp_op: int
    comparison: int
    compound_stmt: int
    continue_stmt: int
    decorated: int
    decorator: int
    decorators: int
    del_stmt: int
    dictsetmaker: int
    dotted_as_name: int
    dotted_as_names: int
    dotted_name: int
    encoding_decl: int
    eval_input: int
    except_clause: int
    exec_stmt: int
    expr: int
    expr_stmt: int
    exprlist: int
    factor: int
    file_input: int
    flow_stmt: int
    for_stmt: int
    funcdef: int
    global_stmt: int
    guard: int
    if_stmt: int
    import_as_name: int
    import_as_names: int
    import_from: int
    import_name: int
    import_stmt: int
    lambdef: int
    listmaker: int
    match_stmt: int
    namedexpr_test: int
    not_test: int
    old_comp_for: int
    old_comp_if: int
    old_comp_iter: int
    old_lambdef: int
    old_test: int
    or_test: int
    parameters: int
    pass_stmt: int
    pattern: int
    patterns: int
    power: int
    print_stmt: int
    raise_stmt: int
    return_stmt: int
    shift_expr: int
    simple_stmt: int
    single_input: int
    sliceop: int
    small_stmt: int
    subject_expr: int
    star_expr: int
    stmt: int
    subscript: int
    subscriptlist: int
    suite: int
    term: int
    test: int
    testlist: int
    testlist1: int
    testlist_gexp: int
    testlist_safe: int
    testlist_star_expr: int
    tfpdef: int
    tfplist: int
    tname: int
    tname_star: int
    trailer: int
    try_stmt: int
    typedargslist: int
    varargslist: int
    vfpdef: int
    vfplist: int
    vname: int
    while_stmt: int
    with_stmt: int
    xor_expr: int
    yield_arg: int
    yield_expr: int
    yield_stmt: int


class _pattern_symbols(Symbols):
    Alternative: int
    Alternatives: int
    Details: int
    Matcher: int
    NegatedUnit: int
    Repeater: int
    Unit: int


python_grammar: Grammar
python_grammar_no_print_statement: Grammar
python_grammar_no_print_statement_no_exec_statement: Grammar
python_grammar_no_print_statement_no_exec_statement_async_keywords: Grammar
python_grammar_no_exec_statement: Grammar
pattern_grammar: Grammar
python_grammar_soft_keywords: Grammar

python_symbols: _python_symbols
pattern_symbols: _pattern_symbols


def initialize(cache_dir: Union[str, "os.PathLike[str]", None] = None) -> None:
    global python_grammar
    global python_grammar_no_print_statement
    global python_grammar_no_print_statement_no_exec_statement
    global python_grammar_no_print_statement_no_exec_statement_async_keywords
    global python_grammar_soft_keywords
    global python_symbols
    global pattern_grammar
    global pattern_symbols

    # The grammar file
    # _GRAMMAR_FILE = os.path.join(os.path.dirname(__file__), "Grammar.txt")
    # _PATTERN_GRAMMAR_FILE = os.path.join(
    #     os.path.dirname(__file__), "PatternGrammar.txt"
    # )

    # Python 2
    # python_grammar = driver.load_packaged_grammar("blib2to3", _GRAMMAR_FILE, cache_dir)
    python_grammar = ParserGenerator(None, stream=io.StringIO("""
# Grammar for 2to3. This grammar supports Python 2.x and 3.x.

# NOTE WELL: You should also follow all the steps listed at
# https://devguide.python.org/grammar/

# Start symbols for the grammar:
#	file_input is a module or sequence of commands read from an input file;
#	single_input is a single interactive statement;
#	eval_input is the input for the eval() and input() functions.
# NB: compound_stmt in single_input is followed by extra NEWLINE!
file_input: (NEWLINE | stmt)* ENDMARKER
single_input: NEWLINE | simple_stmt | compound_stmt NEWLINE
eval_input: testlist NEWLINE* ENDMARKER

decorator: '@' namedexpr_test NEWLINE
decorators: decorator+
decorated: decorators (classdef | funcdef | async_funcdef)
async_funcdef: ASYNC funcdef
funcdef: 'def' NAME parameters ['->' test] ':' suite
parameters: '(' [typedargslist] ')'

# The following definition for typedarglist is equivalent to this set of rules:
#
#     arguments = argument (',' argument)*
#     argument = tfpdef ['=' test]
#     kwargs = '**' tname [',']
#     args = '*' [tname_star]
#     kwonly_kwargs = (',' argument)* [',' [kwargs]]
#     args_kwonly_kwargs = args kwonly_kwargs | kwargs
#     poskeyword_args_kwonly_kwargs = arguments [',' [args_kwonly_kwargs]]
#     typedargslist_no_posonly  = poskeyword_args_kwonly_kwargs | args_kwonly_kwargs
#     typedarglist = arguments ',' '/' [',' [typedargslist_no_posonly]])|(typedargslist_no_posonly)"
#
# It needs to be fully expanded to allow our LL(1) parser to work on it.

typedargslist: tfpdef ['=' test] (',' tfpdef ['=' test])* ',' '/' [
                     ',' [((tfpdef ['=' test] ',')* ('*' [tname_star] (',' tname ['=' test])*
                            [',' ['**' tname [',']]] | '**' tname [','])
                     | tfpdef ['=' test] (',' tfpdef ['=' test])* [','])]
                ] | ((tfpdef ['=' test] ',')* ('*' [tname_star] (',' tname ['=' test])*
                     [',' ['**' tname [',']]] | '**' tname [','])
                     | tfpdef ['=' test] (',' tfpdef ['=' test])* [','])

tname: NAME [':' test]
tname_star: NAME [':' (test|star_expr)]
tfpdef: tname | '(' tfplist ')'
tfplist: tfpdef (',' tfpdef)* [',']

# The following definition for varargslist is equivalent to this set of rules:
#
#     arguments = argument (',' argument )*
#     argument = vfpdef ['=' test]
#     kwargs = '**' vname [',']
#     args = '*' [vname]
#     kwonly_kwargs = (',' argument )* [',' [kwargs]]
#     args_kwonly_kwargs = args kwonly_kwargs | kwargs
#     poskeyword_args_kwonly_kwargs = arguments [',' [args_kwonly_kwargs]]
#     vararglist_no_posonly = poskeyword_args_kwonly_kwargs | args_kwonly_kwargs
#     varargslist = arguments ',' '/' [','[(vararglist_no_posonly)]] | (vararglist_no_posonly)
#
# It needs to be fully expanded to allow our LL(1) parser to work on it.

varargslist: vfpdef ['=' test ](',' vfpdef ['=' test])* ',' '/' [',' [
                     ((vfpdef ['=' test] ',')* ('*' [vname] (',' vname ['=' test])*
                            [',' ['**' vname [',']]] | '**' vname [','])
                            | vfpdef ['=' test] (',' vfpdef ['=' test])* [','])
                     ]] | ((vfpdef ['=' test] ',')*
                     ('*' [vname] (',' vname ['=' test])*  [',' ['**' vname [',']]]| '**' vname [','])
                     | vfpdef ['=' test] (',' vfpdef ['=' test])* [','])

vname: NAME
vfpdef: vname | '(' vfplist ')'
vfplist: vfpdef (',' vfpdef)* [',']

stmt: simple_stmt | compound_stmt
simple_stmt: small_stmt (';' small_stmt)* [';'] NEWLINE
small_stmt: (expr_stmt | print_stmt  | del_stmt | pass_stmt | flow_stmt |
             import_stmt | global_stmt | exec_stmt | assert_stmt)
expr_stmt: testlist_star_expr (annassign | augassign (yield_expr|testlist) |
                     ('=' (yield_expr|testlist_star_expr))*)
annassign: ':' test ['=' (yield_expr|testlist_star_expr)]
testlist_star_expr: (test|star_expr) (',' (test|star_expr))* [',']
augassign: ('+=' | '-=' | '*=' | '@=' | '/=' | '%=' | '&=' | '|=' | '^=' |
            '<<=' | '>>=' | '**=' | '//=')
# For normal and annotated assignments, additional restrictions enforced by the interpreter
print_stmt: 'print' ( [ test (',' test)* [','] ] |
                      '>>' test [ (',' test)+ [','] ] )
del_stmt: 'del' exprlist
pass_stmt: 'pass'
flow_stmt: break_stmt | continue_stmt | return_stmt | raise_stmt | yield_stmt
break_stmt: 'break'
continue_stmt: 'continue'
return_stmt: 'return' [testlist_star_expr]
yield_stmt: yield_expr
raise_stmt: 'raise' [test ['from' test | ',' test [',' test]]]
import_stmt: import_name | import_from
import_name: 'import' dotted_as_names
import_from: ('from' ('.'* dotted_name | '.'+)
              'import' ('*' | '(' import_as_names ')' | import_as_names))
import_as_name: NAME ['as' NAME]
dotted_as_name: dotted_name ['as' NAME]
import_as_names: import_as_name (',' import_as_name)* [',']
dotted_as_names: dotted_as_name (',' dotted_as_name)*
dotted_name: NAME ('.' NAME)*
global_stmt: ('global' | 'nonlocal') NAME (',' NAME)*
exec_stmt: 'exec' expr ['in' test [',' test]]
assert_stmt: 'assert' test [',' test]

compound_stmt: if_stmt | while_stmt | for_stmt | try_stmt | with_stmt | funcdef | classdef | decorated | async_stmt | match_stmt
async_stmt: ASYNC (funcdef | with_stmt | for_stmt)
if_stmt: 'if' namedexpr_test ':' suite ('elif' namedexpr_test ':' suite)* ['else' ':' suite]
while_stmt: 'while' namedexpr_test ':' suite ['else' ':' suite]
for_stmt: 'for' exprlist 'in' testlist_star_expr ':' suite ['else' ':' suite]
try_stmt: ('try' ':' suite
           ((except_clause ':' suite)+
	    ['else' ':' suite]
	    ['finally' ':' suite] |
	   'finally' ':' suite))
with_stmt: 'with' asexpr_test (',' asexpr_test)*  ':' suite

# NB compile.c makes sure that the default except clause is last
except_clause: 'except' ['*'] [test [(',' | 'as') test]]
suite: simple_stmt | NEWLINE INDENT stmt+ DEDENT

# Backward compatibility cruft to support:
# [ x for x in lambda: True, lambda: False if x() ]
# even while also allowing:
# lambda x: 5 if x else 2
# (But not a mix of the two)
testlist_safe: old_test [(',' old_test)+ [',']]
old_test: or_test | old_lambdef
old_lambdef: 'lambda' [varargslist] ':' old_test

namedexpr_test: asexpr_test [':=' asexpr_test]

# This is actually not a real rule, though since the parser is very
# limited in terms of the strategy about match/case rules, we are inserting
# a virtual case (<expr> as <expr>) as a valid expression. Unless a better
# approach is thought, the only side effect of this seem to be just allowing
# more stuff to be parser (which would fail on the ast).
asexpr_test: test ['as' test]

test: or_test ['if' or_test 'else' test] | lambdef
or_test: and_test ('or' and_test)*
and_test: not_test ('and' not_test)*
not_test: 'not' not_test | comparison
comparison: expr (comp_op expr)*
comp_op: '<'|'>'|'=='|'>='|'<='|'<>'|'!='|'in'|'not' 'in'|'is'|'is' 'not'
star_expr: '*' expr
expr: xor_expr ('|' xor_expr)*
xor_expr: and_expr ('^' and_expr)*
and_expr: shift_expr ('&' shift_expr)*
shift_expr: arith_expr (('<<'|'>>') arith_expr)*
arith_expr: term (('+'|'-') term)*
term: factor (('*'|'@'|'/'|'%'|'//') factor)*
factor: ('+'|'-'|'~') factor | power
power: [AWAIT] atom trailer* ['**' factor]
atom: ('(' [yield_expr|testlist_gexp] ')' |
       '[' [listmaker] ']' |
       '{' [dictsetmaker] '}' |
       '`' testlist1 '`' |
       NAME | NUMBER | STRING+ | '.' '.' '.')
listmaker: (namedexpr_test|star_expr) ( old_comp_for | (',' (namedexpr_test|star_expr))* [','] )
testlist_gexp: (namedexpr_test|star_expr) ( old_comp_for | (',' (namedexpr_test|star_expr))* [','] )
lambdef: 'lambda' [varargslist] ':' test
trailer: '(' [arglist] ')' | '[' subscriptlist ']' | '.' NAME
subscriptlist: (subscript|star_expr) (',' (subscript|star_expr))* [',']
subscript: test [':=' test] | [test] ':' [test] [sliceop]
sliceop: ':' [test]
exprlist: (expr|star_expr) (',' (expr|star_expr))* [',']
testlist: test (',' test)* [',']
dictsetmaker: ( ((test ':' asexpr_test | '**' expr)
                 (comp_for | (',' (test ':' asexpr_test | '**' expr))* [','])) |
                ((test [':=' test] | star_expr)
		 (comp_for | (',' (test [':=' test] | star_expr))* [','])) )

classdef: 'class' NAME ['(' [arglist] ')'] ':' suite

arglist: argument (',' argument)* [',']

# "test '=' test" is really "keyword '=' test", but we have no such token.
# These need to be in a single rule to avoid grammar that is ambiguous
# to our LL(1) parser. Even though 'test' includes '*expr' in star_expr,
# we explicitly match '*' here, too, to give it proper precedence.
# Illegal combinations and orderings are blocked in ast.c:
# multiple (test comp_for) arguments are blocked; keyword unpackings
# that precede iterable unpackings are blocked; etc.
argument: ( test [comp_for] |
            test ':=' test [comp_for] |
            test 'as' test |
            test '=' asexpr_test |
	    '**' test |
            '*' test )

comp_iter: comp_for | comp_if
comp_for: [ASYNC] 'for' exprlist 'in' or_test [comp_iter]
comp_if: 'if' old_test [comp_iter]

# As noted above, testlist_safe extends the syntax allowed in list
# comprehensions and generators. We can't use it indiscriminately in all
# derivations using a comp_for-like pattern because the testlist_safe derivation
# contains comma which clashes with trailing comma in arglist.
#
# This was an issue because the parser would not follow the correct derivation
# when parsing syntactically valid Python code. Since testlist_safe was created
# specifically to handle list comprehensions and generator expressions enclosed
# with parentheses, it's safe to only use it in those. That avoids the issue; we
# can parse code like set(x for x in [],).
#
# The syntax supported by this set of rules is not a valid Python 3 syntax,
# hence the prefix "old".
#
# See https://bugs.python.org/issue27494
old_comp_iter: old_comp_for | old_comp_if
old_comp_for: [ASYNC] 'for' exprlist 'in' testlist_safe [old_comp_iter]
old_comp_if: 'if' old_test [old_comp_iter]

testlist1: test (',' test)*

# not used in grammar, but may appear in "node" passed from Parser to Compiler
encoding_decl: NAME

yield_expr: 'yield' [yield_arg]
yield_arg: 'from' test | testlist_star_expr


# 3.10 match statement definition

# PS: normally the grammar is much much more restricted, but
# at this moment for not trying to bother much with encoding the
# exact same DSL in a LL(1) parser, we will just accept an expression
# and let the ast.parse() step of the safe mode to reject invalid
# grammar.

# The reason why it is more restricted is that, patterns are some
# sort of a DSL (more advanced than our LHS on assignments, but
# still in a very limited python subset). They are not really
# expressions, but who cares. If we can parse them, that is enough
# to reformat them.

match_stmt: "match" subject_expr ':' NEWLINE INDENT case_block+ DEDENT

# This is more permissive than the actual version. For example it
# accepts `match *something:`, even though single-item starred expressions
# are forbidden.
subject_expr: (namedexpr_test|star_expr) (',' (namedexpr_test|star_expr))* [',']

# cases
case_block: "case" patterns [guard] ':' suite
guard: 'if' namedexpr_test
patterns: pattern (',' pattern)* [',']
pattern: (expr|star_expr) ['as' expr]
    """)).make_grammar()
    python_grammar.version = (2, 0)

    soft_keywords = python_grammar.soft_keywords.copy()
    python_grammar.soft_keywords.clear()

    python_symbols = _python_symbols(python_grammar)

    # Python 2 + from __future__ import print_function
    python_grammar_no_print_statement = python_grammar.copy()
    del python_grammar_no_print_statement.keywords["print"]

    # Python 3.0-3.6
    python_grammar_no_print_statement_no_exec_statement = python_grammar.copy()
    del python_grammar_no_print_statement_no_exec_statement.keywords["print"]
    del python_grammar_no_print_statement_no_exec_statement.keywords["exec"]
    python_grammar_no_print_statement_no_exec_statement.version = (3, 0)

    # Python 3.7+
    python_grammar_no_print_statement_no_exec_statement_async_keywords = (
        python_grammar_no_print_statement_no_exec_statement.copy()
    )
    python_grammar_no_print_statement_no_exec_statement_async_keywords.async_keywords = (
        True
    )
    python_grammar_no_print_statement_no_exec_statement_async_keywords.version = (3, 7)

    # Python 3.10+
    python_grammar_soft_keywords = (
        python_grammar_no_print_statement_no_exec_statement_async_keywords.copy()
    )
    python_grammar_soft_keywords.soft_keywords = soft_keywords
    python_grammar_soft_keywords.version = (3, 10)

    # pattern_grammar = driver.load_packaged_grammar(
    #     "blib2to3", _PATTERN_GRAMMAR_FILE, cache_dir
    # )
    pattern_grammar = ParserGenerator(None, stream=io.StringIO("""
# Copyright 2006 Google, Inc. All Rights Reserved.
# Licensed to PSF under a Contributor Agreement.

# A grammar to describe tree matching patterns.
# Not shown here:
# - 'TOKEN' stands for any token (leaf node)
# - 'any' stands for any node (leaf or interior)
# With 'any' we can still specify the sub-structure.

# The start symbol is 'Matcher'.

Matcher: Alternatives ENDMARKER

Alternatives: Alternative ('|' Alternative)*

Alternative: (Unit | NegatedUnit)+

Unit: [NAME '='] ( STRING [Repeater]
                 | NAME [Details] [Repeater]
                 | '(' Alternatives ')' [Repeater]
                 | '[' Alternatives ']'
		 )

NegatedUnit: 'not' (STRING | NAME [Details] | '(' Alternatives ')')

Repeater: '*' | '+' | '{' NUMBER [',' NUMBER] '}'

Details: '<' Alternatives '>'
    """)).make_grammar()
    pattern_symbols = _pattern_symbols(pattern_grammar)
