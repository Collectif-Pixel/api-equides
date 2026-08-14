import { writeFile, readFile } from 'node:fs/promises'

const SOURCE = process.env.OPENAPI_URL ?? 'https://api-equides.org/openapi.json'
const CIBLE = new URL('./openapi.json', import.meta.url)

const echec = (raison) => {
  console.error(`✗ ${raison}`)
  process.exitCode = 1
}

try {
  const reponse = await fetch(SOURCE, { headers: { accept: 'application/json' } })
  if (!reponse.ok) throw new Error(`HTTP ${reponse.status}`)

  const document = await reponse.json()
  if (document.openapi !== '3.1.0' || !document.paths) {
    throw new Error('document reçu non conforme')
  }

  const routes = Object.keys(document.paths).length
  await writeFile(CIBLE, `${JSON.stringify(document, null, 2)}\n`)
  console.log(`✓ ${SOURCE} → openapi.json (${routes} routes, v${document.info.version})`)
} catch (erreur) {
  try {
    const repli = JSON.parse(await readFile(CIBLE, 'utf8'))
    console.warn(
      `⚠ ${SOURCE} injoignable (${erreur.message}) — repli sur l'exemplaire ` +
        `versionné, v${repli.info.version}. La référence peut être périmée.`,
    )
  } catch {
    echec(`${SOURCE} injoignable (${erreur.message}) et aucun exemplaire versionné`)
  }
}
