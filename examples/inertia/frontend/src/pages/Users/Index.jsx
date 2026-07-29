import {
  Deferred,
  Head,
  InfiniteScroll,
  Link,
  router,
  usePage,
} from '@inertiajs/react'
import { useEffect, useState } from 'react'

export default function Users({ users, stats, activity, navigation }) {
  const { flash } = usePage()
  const [notice, setNotice] = useState(flash?.notice)

  // InfiniteScroll can start a partial reload immediately when the list fits
  // in the viewport. Keep the last non-empty flash value across those
  // background requests so the notice remains readable.
  useEffect(() => {
    if (flash?.notice) {
      setNotice(flash.notice)
    }
  }, [flash?.notice])

  return (
    <>
      <Head title="Users" />
      <nav>
        <Link href="/">Home</Link>
        <Link href="/users/create">Create user</Link>
      </nav>
      <h1>Users</h1>
      {notice && <p className="flash">{notice}</p>}
      <div className="actions">
        <button onClick={() => router.reload({ only: ['stats'] })}>
          Reload only stats
        </button>
        <button onClick={() => router.reload({ reset: ['navigation'] })}>
          Refresh once navigation
        </button>
      </div>
      <p>
        Once navigation resolution #{navigation.resolution}:{' '}
        {navigation.items.join(', ')}
      </p>
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
        <p>Stats resolution: #{stats?.resolution}</p>
        <p>Deferred merged activity: {activity?.join(', ')}</p>
      </Deferred>
      <p className="user-count">
        Showing {users.length} of {stats?.total ?? '...'} users
      </p>
      <p>Scroll inside the list to load the next page.</p>
      <div className="user-scroll">
        <InfiniteScroll
          data="users"
          as="ul"
          className="user-list"
          buffer={80}
          preserveUrl
          next={({ hasMore, loading }) => (
            <p className="scroll-status">
              {loading
                ? 'Loading more users...'
                : hasMore
                  ? 'Keep scrolling to load more'
                  : 'All users loaded'}
            </p>
          )}
        >
          {users.map((user) => (
            <li key={user.id}>{user.name}</li>
          ))}
        </InfiniteScroll>
      </div>
    </>
  )
}
