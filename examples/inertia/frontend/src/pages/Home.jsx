import { Head, Link, usePage } from '@inertiajs/react'

export default function Home({ greeting }) {
  const { auth, errors } = usePage().props

  return (
    <>
      <Head title="Topcoat Inertia" />
      <nav>
        <Link href="/">Home</Link>
        <Link href="/users">Users</Link>
        <Link href="/users/create">Create user</Link>
        <Link href="/optimistic">Optimistic updates</Link>
      </nav>
      <h1>{greeting}</h1>
      <p>Shared auth: {auth.name}</p>
      <p>Reserved validation keys: {Object.keys(errors).length}</p>
    </>
  )
}
