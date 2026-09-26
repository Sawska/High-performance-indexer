import { render, screen } from '@testing-library/react'
import type { ReactElement } from 'react'
import { MemoryRouter, Route, Routes } from 'react-router'
import { LocationProbe } from './LocationProbe'

export function renderRoute(path: string, url: string, element: ReactElement) {
  return render(
    <MemoryRouter initialEntries={[url]}>
      <Routes>
        <Route path={path} element={element} />
      </Routes>
      <LocationProbe />
    </MemoryRouter>,
  )
}

export function tiles(container: HTMLElement): Record<string, { value: string; sub: string | null }> {
  return Object.fromEntries(
    [...container.querySelectorAll('.tile')].map((el) => [
      el.querySelector('.label')?.textContent ?? '',
      { value: el.querySelector('.value')?.textContent ?? '', sub: el.querySelector('.sub')?.textContent ?? null },
    ]),
  )
}

export function bodyRows(table: HTMLElement): string[][] {
  return [...table.querySelectorAll('tbody tr')].map((tr) => [...tr.querySelectorAll('td')].map((td) => td.textContent ?? ''))
}

export function tableWithHeader(name: string): HTMLElement {
  const table = screen.getByRole('columnheader', { name }).closest('table')
  if (!table) throw new Error(`no table with header ${name}`)
  return table
}
