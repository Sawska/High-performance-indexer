import { useLocation } from 'react-router'

/** Renders the router's current path and query, for asserting navigation. */
export function LocationProbe() {
  const location = useLocation()
  return <output data-testid="location">{location.pathname + location.search}</output>
}
