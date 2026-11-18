import { describe, expect, it } from 'vitest'
import { compareCells, delimiterOf, numericColumns, parseCsv } from './csv'

describe('delimiterOf', () => {
  it('picks the separator of the first line, outside quotes', () => {
    expect(delimiterOf('a,b,c\n1,2,3')).toBe(',')
    expect(delimiterOf('nom;prix;"a,b,c"\nx;1;y')).toBe(';')
    expect(delimiterOf('a\tb\n1\t2')).toBe('\t')
  })

  it('uses tabs for tsv files', () => {
    expect(delimiterOf('a,b\tc', 'export.tsv')).toBe('\t')
  })
})

describe('parseCsv', () => {
  it('reads quoted cells with separators, quotes and line breaks', () => {
    expect(parseCsv('﻿a,"b,1","say ""hi"""\r\n"multi\nline",,x\n', ',')).toEqual([
      ['a', 'b,1', 'say "hi"'],
      ['multi\nline', '', 'x'],
    ])
  })

  it('keeps a last line without a line break', () => {
    expect(parseCsv('a;b\n1;2', ';')).toEqual([
      ['a', 'b'],
      ['1', '2'],
    ])
  })
})

describe('numericColumns', () => {
  it('marks columns whose filled cells are all numbers', () => {
    const rows = [
      ['12,50', 'abc', '1 234'],
      ['', '3', '42 %'],
      ['-3.5', 'x', '7'],
    ]
    expect(numericColumns(rows, 3)).toEqual([true, false, true])
  })
})

describe('compareCells', () => {
  it('sorts numbers by value with French or English decimals', () => {
    expect(['10', '9,5', '1 200', '100.25'].sort((a, b) => compareCells(a, b, true))).toEqual(['9,5', '10', '100.25', '1 200'])
  })

  it('sorts text naturally', () => {
    expect(['item10', 'Item2', 'item1'].sort((a, b) => compareCells(a, b))).toEqual(['item1', 'Item2', 'item10'])
  })
})
