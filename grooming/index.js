const got = require('got')
const entries = require('../entries.json')

const DEFAULT_TIMEOUT = 15000

const chopUrl = url => {
    const chopped = /(https?):\/\/(www)?\.?(.+)/gim.exec(url)
    return (chopped && chopped[3]) || url
}

const createPossibleUrls = partialUrl => [
    `https://${partialUrl}`,
    `http://${partialUrl}`,
    `https://www.${partialUrl}`,
    `http://www.${partialUrl}`,
    partialUrl, // AS IS
]

const checkUrlStatus = async url => {
    console.log(`Checking url "${url}"`)
    try {
        await got(url, { timeout: DEFAULT_TIMEOUT })
        return url
    } catch (error) {
        // console.error(`Error for url "${url}:"`, error)
        return null
    }
}

const selectFirstValidUrl =  urls => {
    const checkedUrls = await Promise.all(urls.map(checkUrlStatus))
    return checkedUrls.find(x => x)
}

async function sanitizeAndOrderURLs(urls) {
    const choppedUrls = urls.map(chopUrl)
    const possibleUrlSets = choppedUrls.sort().map(createPossibleUrls)
    const possibleValidUrls = await Promise.all(possibleUrlSets.map(selectFirstValidUrl))
    return possibleValidUrls.filter(x => x)
}

async function main() {
    const sanitizedUrls = await sanitizeAndOrderURLs(entries)
    console.log(sanitizedUrls)
    console.log(`${entries.length} entries`)
    console.log(`${sanitizedUrls.length} valid urls`)
}
main().catch(err => {
    console.error(err)
    process.exit(1)
})
