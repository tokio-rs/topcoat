import { Form, Head, Link } from '@inertiajs/react'

export default function Optimistic({ optimisticUsers }) {
  return (
    <>
      <Head title="Optimistic updates" />
      <nav>
        <Link href="/">Home</Link>
        <Link href="/users">Users</Link>
        <Link href="/users/create">Create user</Link>
      </nav>
      <h1>Optimistic updates</h1>
      <p>
        Submit the form to add a temporary row immediately. The server response
        then replaces it with the persisted row or rolls it back on validation
        failure.
      </p>
      <Form
        action="/optimistic"
        method="post"
        errorBag="optimisticUser"
        resetOnSuccess
        disableWhileProcessing
        optimistic={(props, data) => ({
          optimisticUsers: [
            {
              id: `optimistic-${Date.now()}`,
              name: String(data.name).trim(),
              age: String(data.age),
              saving: true,
            },
            ...props.optimisticUsers,
          ],
        })}
      >
        {({ errors, processing }) => (
          <div className="optimistic-form">
            <label>
              Name
              <input name="name" placeholder="Lin" />
              {errors.name && <span className="error">{errors.name}</span>}
            </label>
            <label>
              Age
              <input name="age" type="number" min="1" max="120" />
              {errors.age && <span className="error">{errors.age}</span>}
            </label>
            <button type="submit">
              {processing ? 'Saving...' : 'Add immediately'}
            </button>
          </div>
        )}
      </Form>
      <ul className="optimistic-list">
        {optimisticUsers.map((user) => (
          <li key={user.id}>
            <span>{user.name}</span>
            <span>Age {user.age}</span>
            {user.saving && <strong>Saving...</strong>}
          </li>
        ))}
      </ul>
    </>
  )
}
