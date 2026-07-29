import { Head, Link, useForm } from '@inertiajs/react'

export default function CreateUser() {
  const form = useForm({ name: '' })

  function submit(event) {
    event.preventDefault()
    form.post('/users', { errorBag: 'createUser' })
  }

  return (
    <>
      <Head title="Create user" />
      <Link href="/users">Back to users</Link>
      <h1>Create user</h1>
      <form onSubmit={submit}>
        <label>
          Name{' '}
          <input
            value={form.data.name}
            onChange={(event) => form.setData('name', event.target.value)}
          />
        </label>
        {form.errors.name && <p className="error">{form.errors.name}</p>}
        <button disabled={form.processing}>Create</button>
      </form>
    </>
  )
}
