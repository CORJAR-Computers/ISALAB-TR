import { describe, it, expect } from 'vitest'
import { formatCOP, formatDate, formatAge, cn, formatDateTime, localDateISO } from './utils'

describe('formatCOP', () => {
  it('formats a whole number as Colombian Pesos', () => {
    expect(formatCOP(1500000)).toContain('1.500.000')
  })

  it('formats a small number', () => {
    expect(formatCOP(4500)).toContain('4.500')
  })

  it('formats zero', () => {
    expect(formatCOP(0)).toContain('0')
  })

  it('includes the $ sign', () => {
    expect(formatCOP(100)).toContain('$')
  })
})

describe('formatDate', () => {
  it('formats a valid date string', () => {
    const result = formatDate('2024-03-15')
    expect(result).toContain('2024')
    expect(result).not.toBe('—')
  })

  it('returns dash for null/undefined', () => {
    expect(formatDate(null)).toBe('—')
    expect(formatDate(undefined)).toBe('—')
    expect(formatDate('')).toBe('—')
  })

  it('returns original value for invalid date', () => {
    expect(formatDate('not-a-date')).toBe('not-a-date')
  })
})

describe('formatDateTime', () => {
  it('formats a datetime string', () => {
    const result = formatDateTime('2024-03-15 10:30:00')
    expect(result).toContain('2024')
    expect(result).not.toBe('—')
  })

  it('formats a date-only string', () => {
    const result = formatDateTime('2024-03-15')
    expect(result).toContain('2024')
  })

  it('returns dash for null/undefined', () => {
    expect(formatDateTime(null)).toBe('—')
    expect(formatDateTime(undefined)).toBe('—')
  })
})

describe('formatAge', () => {
  it('returns dash for null/undefined', () => {
    expect(formatAge(null)).toBe('—')
    expect(formatAge(undefined)).toBe('—')
  })

  it('returns months for young animals', () => {
    const recent = new Date()
    recent.setMonth(recent.getMonth() - 6)
    const dateStr = recent.toISOString().split('T')[0]
    const result = formatAge(dateStr)
    expect(result).toContain('mes')
  })

  it('returns years for older animals', () => {
    const result = formatAge('2020-01-01')
    expect(result).toMatch(/\d/)
  })
})

describe('localDateISO', () => {
  it('usa la fecha local aunque la UTC ya sea la del día siguiente', () => {
    // 2026-01-16 01:00 UTC = 2026-01-15 20:00 en Colombia (UTC-5): el bug que
    // hacía que la cola de recepción apuntara al día siguiente.
    const afterSevenPm = new Date('2026-01-16T01:00:00Z')
    expect(afterSevenPm.toISOString().slice(0, 10)).toBe('2026-01-16')
    expect(localDateISO(afterSevenPm, 'America/Bogota')).toBe('2026-01-15')
  })

  it('cambia de día a medianoche local, no a medianoche UTC', () => {
    // 04:59 UTC sigue siendo el 15 en Bogotá; 05:00 UTC ya es el 16.
    expect(
      localDateISO(new Date('2026-01-16T04:59:00Z'), 'America/Bogota'),
    ).toBe('2026-01-15')
    expect(
      localDateISO(new Date('2026-01-16T05:00:00Z'), 'America/Bogota'),
    ).toBe('2026-01-16')
  })

  it('devuelve YYYY-MM-DD con ceros a la izquierda', () => {
    expect(localDateISO(new Date('2026-03-05T12:00:00Z'), 'UTC')).toBe(
      '2026-03-05',
    )
  })

  it('por defecto usa la zona del sistema', () => {
    // Construido con el constructor local: 23:30 del 15 de enero.
    expect(localDateISO(new Date(2026, 0, 15, 23, 30))).toBe('2026-01-15')
  })
})

describe('cn', () => {
  it('merges class names', () => {
    expect(cn('a', 'b')).toBe('a b')
  })

  it('handles conditional classes', () => {
    expect(cn('a', false && 'b', 'c')).toBe('a c')
  })

  it('handles empty input', () => {
    expect(cn()).toBe('')
  })

  it('deduplicates tailwind classes', () => {
    expect(cn('px-2', 'px-4')).toBe('px-4')
  })
})
