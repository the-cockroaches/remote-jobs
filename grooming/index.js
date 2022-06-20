import fs from 'fs'
import got from 'got'

const DEFAULT_TIMEOUT = 30000
const chopUrl = url => {
    const chopped = /(https?):\/\/(www)?\.?(.+)/gim.exec(url.trim())
    return (chopped && chopped[3]) || url
}

const buildURLVariations = url => {
    const partialUrl = chopUrl(url)
    // NOTE: order matters here
    const variations = [
        `https://${partialUrl}`,
        `http://${partialUrl}`,
        `https://www.${partialUrl}`,
        `http://www.${partialUrl}`,
    ]
    if (!variations.find(x => x === url)) {
        variations.push(url) // AS IS
    }
    return variations
}

class Progress {
    #total = 0
    #completed = 0

    add() {
        this.#total++
    }
    complete() {
        this.#completed++
    }
    get percent() {
        return this.#total ? (this.#completed / this.#total) * 100 : 0
    }
}

let hitCount = 0

const hitUrl = progress => async url => {
    progress.add()
    const time = Date.now()
    try {
        console.log(`hitCount: ${++hitCount} - hitting ${url}`)
        await got(url, { timeout: { response: DEFAULT_TIMEOUT } })
        return url
    } catch (error) {
        console.error(error.toString(), `error for url "${url}"`)
        return null
    } finally {
        progress.complete()
        console.log(
            `${progress.percent.toFixed(2)}% - ${Math.round((Date.now() - time) / 1000)}s - "${url}"`
        )
    }
}

const checkUrl = progress => async url => {
    progress.add()
    const variations = buildURLVariations(url)
    const hitUrls = await Promise.all(variations.map(hitUrl(progress)))
    const validVariation = hitUrls.find(Boolean)
    progress.complete()
    return { isValid: !!validVariation, url: validVariation, originalUrl: url }
}

async function main() {
    const data = fs.readFileSync('../job-links.txt').toString()
    const urls = data
        .split(/\r?\n/)
        .map(x => x.trim())
        .filter(Boolean)
        .sort()

    const progress = new Progress()
    const urlStatusList = await Promise.all(urls.map(checkUrl(progress)))

    const validUrls = new Set()
    const invalidUrls = new Set()

    urlStatusList.forEach(({ isValid, url, originalUrl }) => {
        if (isValid) {
            validUrls.add(url)
        } else {
            invalidUrls.add(originalUrl)
        }
    })

    console.log('valid urls', validUrls)
    console.log('invalid urls', invalidUrls)

    fs.writeFileSync('../groomed-links.txt', Array.from(validUrls).sort().join('\n'))
}

await main()
