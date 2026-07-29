import {
  Deferred,
  Head,
  InfiniteScroll,
  Link,
  router,
} from '@inertiajs/react'

export default function Users({ users, stats, activity, navigation, flash }) {
  return (
    <>
      <Head title="Users" />
      <nav>
        <Link href="/">Home</Link>
        <Link href="/users/create">Create user</Link>
      </nav>
      <h1>Users</h1>
      {flash?.notice && <p className="flash">{flash.notice}</p>}
      <div className="actions">
        <button onClick={() => router.reload({ only: ['stats'] })}>
          Partial stats reload
        </button>
        <button
          onClick={() =>
            router.visit('/users?refresh_navigation=1', {
              only: ['navigation'],
            })
          }
        >
          Force navigation refresh
        </button>
      </div>
      <p>Once navigation: {navigation.join(', ')}</p>
      <Deferred
        data={['stats', 'activity']}
        fallback={<p>Loading deferred data...</p>}
        rescue={({ reloading }) => (
          <button
            disabled={reloading}
            onClick={() => router.reload({ only: ['stats', 'activity'] })}
          >
            Retry deferred data
          </button>
        )}
      >
        <p>Total users: {stats?.total}</p>
        <p>Deferred merged activity: {activity?.join(', ')}</p>
      </Deferred>
      <InfiniteScroll data="users" buffer={100}>
        <ul>
          {users.map((user) => (
            <li key={user.id}>{user.name}</li>
          ))}
        </ul>
      </InfiniteScroll>
    </>
  )
}
