/** One renderer's active call owns its microphone until the call view is gone. */
const owners = new Set<symbol>()

export const callOwnsMicrophone = () => owners.size > 0

export function acquireCallMicrophone(): () => void {
  const owner = Symbol('voice call')
  owners.add(owner)
  return () => { owners.delete(owner) }
}
