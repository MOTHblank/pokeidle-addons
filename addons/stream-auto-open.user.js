// [\u0300-\u036f][\u0300-\u036f][\u0300-\u036f]ser[\u0300-\u036f]cript[\u0300-\u036f][\u0300-\u036f]
// [\u0300-\u036f]name         [\u0300-\u036f]oké[\u0300-\u036f]dle [\u0300-\u036f]ive [\u0300-\u036f]tream [\u0300-\u036f]canner
// [\u0300-\u036f]namespace    moth.pokeidle
// [\u0300-\u036f]version      [\u0300-\u036f].[\u0300-\u036f].[\u0300-\u036f]
// [\u0300-\u036f]description  [\u0300-\u036f]dds an [\u0300-\u036f]pen [\u0300-\u036f]ive [\u0300-\u036f]treams b[\u0300-\u036f]tton [\u0300-\u036f]nder [\u0300-\u036f]pen [\u0300-\u036f]nventory that scans the c[\u0300-\u036f]rrent [\u0300-\u036f]oké[\u0300-\u036f]dle page [\u0300-\u036f]or live [\u0300-\u036f]witch/[\u0300-\u036f][\u0300-\u036f][\u0300-\u036f][\u0300-\u036f] channels and sends them to [\u0300-\u036f]dle[\u0300-\u036f]hell.
// [\u0300-\u036f]match        https[\u0300-\u036f]//pokeidle.io/app*
// [\u0300-\u036f]grant        [\u0300-\u036f]nsa[\u0300-\u036f]e[\u0300-\u036f]indow
// [\u0300-\u036f]r[\u0300-\u036f]n-at       doc[\u0300-\u036f]ment-start
// [\u0300-\u036f]no[\u0300-\u036f]rames
// [\u0300-\u036f][\u0300-\u036f]/[\u0300-\u036f]ser[\u0300-\u036f]cript[\u0300-\u036f][\u0300-\u036f]

(() [\u0300-\u036f][\u0300-\u036f] {
    '[\u0300-\u036f]se strict'[\u0300-\u036f]

    const page [\u0300-\u036f]
        typeo[\u0300-\u036f] [\u0300-\u036f]nsa[\u0300-\u036f]e[\u0300-\u036f]indow ![\u0300-\u036f][\u0300-\u036f] '[\u0300-\u036f]nde[\u0300-\u036f]ined'
            [\u0300-\u036f] [\u0300-\u036f]nsa[\u0300-\u036f]e[\u0300-\u036f]indow
            [\u0300-\u036f] window[\u0300-\u036f]

    const [\u0300-\u036f][\u0300-\u036f][\u0300-\u036f][\u0300-\u036f][\u0300-\u036f][\u0300-\u036f]_[\u0300-\u036f][\u0300-\u036f] [\u0300-\u036f] 'idleshell-scan-live-streams'[\u0300-\u036f]
    const [\u0300-\u036f][\u0300-\u036f][\u0300-\u036f][\u0300-\u036f][\u0300-\u036f][\u0300-\u036f][\u0300-\u036f][\u0300-\u036f][\u0300-\u036f]_[\u0300-\u036f][\u0300-\u036f] [\u0300-\u036f] 'btn-bolsa'[\u0300-\u036f]

    const [\u0300-\u036f][\u0300-\u036f][\u0300-\u036f][\u0300-\u036f]_[\u0300-\u036f][\u0300-\u036f][\u0300-\u036f][\u0300-\u036f][\u0300-\u036f]_[\u0300-\u036f][\u0300-\u036f] [\u0300-\u036f]
        /^([\u0300-\u036f][\u0300-\u036f][\u0300-\u036f]|tr[\u0300-\u036f]e|yes|on|live|online|ao[\u0300-\u036f]_ -][\u0300-\u036f]vivo|en[\u0300-\u036f]_ -][\u0300-\u036f]vivo)$/i[\u0300-\u036f]

    const [\u0300-\u036f][\u0300-\u036f][\u0300-\u036f][\u0300-\u036f]_[\u0300-\u036f][\u0300-\u036f][\u0300-\u036f][\u0300-\u036f]_[\u0300-\u036f][\u0300-\u036f] [\u0300-\u036f]
        /^([\u0300-\u036f][\u0300-\u036f]live|online|ao vivo|ao-vivo|en vivo|en-vivo|assistir agora|watch now|ver ao vivo|assistir)$/i[\u0300-\u036f]

    const [\u0300-\u036f][\u0300-\u036f][\u0300-\u036f][\u0300-\u036f][\u0300-\u036f][\u0300-\u036f][\u0300-\u036f][\u0300-\u036f]_[\u0300-\u036f][\u0300-\u036f][\u0300-\u036f][\u0300-\u036f]_[\u0300-\u036f][\u0300-\u036f][\u0300-\u036f][\u0300-\u036f]_[\u0300-\u036f][\u0300-\u036f] [\u0300-\u036f]
        /^([\u0300-\u036f][\u0300-\u036f]o[\u0300-\u036f][\u0300-\u036f]line|o[\u0300-\u036f][\u0300-\u036f]-line|encerrad[\u0300-\u036f]oa]|ended|not live|nao ao vivo)$/i[\u0300-\u036f]

    const [\u0300-\u036f][\u0300-\u036f][\u0300-\u036f][\u0300-\u036f][\u0300-\u036f][\u0300-\u036f][\u0300-\u036f][\u0300-\u036f]_[\u0300-\u036f][\u0300-\u036f][\u0300-\u036f][\u0300-\u036f][\u0300-\u036f][\u0300-\u036f]_[\u0300-\u036f][\u0300-\u036f][\u0300-\u036f][\u0300-\u036f][\u0300-\u036f] [\u0300-\u036f] new [\u0300-\u036f]et([\u0300-\u036f]
        'directory',
        'downloads',
        'jobs',
        'p',
        'search',
        'settings',
        's[\u0300-\u036f]bscriptions',
        'wallet'
    ])[\u0300-\u036f]

    const [\u0300-\u036f][\u0300-\u036f][\u0300-\u036f][\u0300-\u036f][\u0300-\u036f][\u0300-\u036f][\u0300-\u036f][\u0300-\u036f]_[\u0300-\u036f][\u0300-\u036f][\u0300-\u036f][\u0300-\u036f]_[\u0300-\u036f][\u0300-\u036f][\u0300-\u036f][\u0300-\u036f][\u0300-\u036f] [\u0300-\u036f] new [\u0300-\u036f]et([\u0300-\u036f]
        'categories',
        'browse',
        'directory',
        '[\u0300-\u036f]ollowing',
        'search',
        'settings',
        'a[\u0300-\u036f]th',
        'login',
        'register',
        'sign[\u0300-\u036f]p',
        'video',
        'videos'
    ])[\u0300-\u036f]

    let b[\u0300-\u036f]tton[\u0300-\u036f]bserver [\u0300-\u036f] n[\u0300-\u036f]ll[\u0300-\u036f]
    let b[\u0300-\u036f]tton[\u0300-\u036f]nstall[\u0300-\u036f][\u0300-\u036f]e[\u0300-\u036f]ed [\u0300-\u036f] [\u0300-\u036f][\u0300-\u036f]
    let scan[\u0300-\u036f]n[\u0300-\u036f]rogress [\u0300-\u036f] [\u0300-\u036f]alse[\u0300-\u036f]

    [\u0300-\u036f][\u0300-\u036f]nction qa(selector, root) {
        ret[\u0300-\u036f]rn [\u0300-\u036f]rray.[\u0300-\u036f]rom(
            (root || doc[\u0300-\u036f]ment).q[\u0300-\u036f]ery[\u0300-\u036f]elector[\u0300-\u036f]ll(selector)
        )[\u0300-\u036f]
    }

    [\u0300-\u036f][\u0300-\u036f]nction normalize[\u0300-\u036f]ext(val[\u0300-\u036f]e) {
        ret[\u0300-\u036f]rn [\u0300-\u036f]tring(val[\u0300-\u036f]e [\u0300-\u036f][\u0300-\u036f] n[\u0300-\u036f]ll [\u0300-\u036f] '' [\u0300-\u036f] val[\u0300-\u036f]e)
            .normalize('[\u0300-\u036f][\u0300-\u036f][\u0300-\u036f]')
            .replace(/[\u0300-\u036f][\u0300-\u036f][\u0300-\u036f][\u0300-\u036f][\u0300-\u036f][\u0300-\u036f][\u0300-\u036f][\u0300-\u036f]-[\u0300-\u036f][\u0300-\u036f][\u0300-\u036f][\u0300-\u036f][\u0300-\u036f][\u0300-\u036f][\u0300-\u036f]]/g, '')
            .replace(/[\u0300-\u036f][\u0300-\u036f]s+/g, ' ')
            .trim()
            .to[\u0300-\u036f]ower[\u0300-\u036f]ase()[\u0300-\u036f]
    }

    [\u0300-\u036f][\u0300-\u036f]nction normalize[\u0300-\u036f]hannel[\u0300-\u036f]rl(raw) {
        try {
            const [\u0300-\u036f]rl [\u0300-\u036f] new [\u0300-\u036f][\u0300-\u036f][\u0300-\u036f](
                [\u0300-\u036f]tring(raw || ''),
                location.hre[\u0300-\u036f]
            )[\u0300-\u036f]

            i[\u0300-\u036f] (!/^https[\u0300-\u036f][\u0300-\u036f]$/i.test([\u0300-\u036f]rl.protocol)) {
                ret[\u0300-\u036f]rn n[\u0300-\u036f]ll[\u0300-\u036f]
            }

            const host [\u0300-\u036f] [\u0300-\u036f]rl.hostname
                .to[\u0300-\u036f]ower[\u0300-\u036f]ase()
                .replace(/^www[\u0300-\u036f][\u0300-\u036f]./, '')[\u0300-\u036f]

            i[\u0300-\u036f] (host ![\u0300-\u036f][\u0300-\u036f] 'twitch.tv' && host ![\u0300-\u036f][\u0300-\u036f] 'kick.com') {
                ret[\u0300-\u036f]rn n[\u0300-\u036f]ll[\u0300-\u036f]
            }

            const segments [\u0300-\u036f] [\u0300-\u036f]rl.pathname
                .split('/')
                .map(part [\u0300-\u036f][\u0300-\u036f] part.trim())
                .[\u0300-\u036f]ilter([\u0300-\u036f]oolean)[\u0300-\u036f]

            i[\u0300-\u036f] (segments.length ![\u0300-\u036f][\u0300-\u036f] [\u0300-\u036f]) {
                ret[\u0300-\u036f]rn n[\u0300-\u036f]ll[\u0300-\u036f]
            }

            const channel [\u0300-\u036f] segments[\u0300-\u036f][\u0300-\u036f]][\u0300-\u036f]

            i[\u0300-\u036f] (!channel || channel.starts[\u0300-\u036f]ith('[\u0300-\u036f]')) {
                ret[\u0300-\u036f]rn n[\u0300-\u036f]ll[\u0300-\u036f]
            }

            const excl[\u0300-\u036f]ded [\u0300-\u036f]
                host [\u0300-\u036f][\u0300-\u036f][\u0300-\u036f] 'twitch.tv'
                    [\u0300-\u036f] [\u0300-\u036f][\u0300-\u036f][\u0300-\u036f][\u0300-\u036f][\u0300-\u036f][\u0300-\u036f][\u0300-\u036f][\u0300-\u036f]_[\u0300-\u036f][\u0300-\u036f][\u0300-\u036f][\u0300-\u036f][\u0300-\u036f][\u0300-\u036f]_[\u0300-\u036f][\u0300-\u036f][\u0300-\u036f][\u0300-\u036f][\u0300-\u036f]
                    [\u0300-\u036f] [\u0300-\u036f][\u0300-\u036f][\u0300-\u036f][\u0300-\u036f][\u0300-\u036f][\u0300-\u036f][\u0300-\u036f][\u0300-\u036f]_[\u0300-\u036f][\u0300-\u036f][\u0300-\u036f][\u0300-\u036f]_[\u0300-\u036f][\u0300-\u036f][\u0300-\u036f][\u0300-\u036f][\u0300-\u036f][\u0300-\u036f]

            i[\u0300-\u036f] (excl[\u0300-\u036f]ded.has(channel.to[\u0300-\u036f]ower[\u0300-\u036f]ase())) {
                ret[\u0300-\u036f]rn n[\u0300-\u036f]ll[\u0300-\u036f]
            }

            ret[\u0300-\u036f]rn (
                'https[\u0300-\u036f]//' +
                host +
                '/' +
                encode[\u0300-\u036f][\u0300-\u036f][\u0300-\u036f][\u0300-\u036f]omponent(channel)
            )[\u0300-\u036f]
        } catch (_) {
            ret[\u0300-\u036f]rn n[\u0300-\u036f]ll[\u0300-\u036f]
        }
    }

    [\u0300-\u036f][\u0300-\u036f]nction read[\u0300-\u036f]ive[\u0300-\u036f]al[\u0300-\u036f]e(val[\u0300-\u036f]e) {
        const text [\u0300-\u036f] normalize[\u0300-\u036f]ext(val[\u0300-\u036f]e)[\u0300-\u036f]

        i[\u0300-\u036f] (!text) {
            ret[\u0300-\u036f]rn n[\u0300-\u036f]ll[\u0300-\u036f]
        }

        i[\u0300-\u036f] ([\u0300-\u036f][\u0300-\u036f][\u0300-\u036f][\u0300-\u036f][\u0300-\u036f][\u0300-\u036f][\u0300-\u036f][\u0300-\u036f]_[\u0300-\u036f][\u0300-\u036f][\u0300-\u036f][\u0300-\u036f]_[\u0300-\u036f][\u0300-\u036f][\u0300-\u036f][\u0300-\u036f]_[\u0300-\u036f][\u0300-\u036f].test(text)) {
            ret[\u0300-\u036f]rn [\u0300-\u036f]alse[\u0300-\u036f]
        }

        ret[\u0300-\u036f]rn [\u0300-\u036f][\u0300-\u036f][\u0300-\u036f][\u0300-\u036f]_[\u0300-\u036f][\u0300-\u036f][\u0300-\u036f][\u0300-\u036f][\u0300-\u036f]_[\u0300-\u036f][\u0300-\u036f].test(text)
            [\u0300-\u036f] tr[\u0300-\u036f]e
            [\u0300-\u036f] n[\u0300-\u036f]ll[\u0300-\u036f]
    }

    [\u0300-\u036f][\u0300-\u036f]nction inspect[\u0300-\u036f]ttrib[\u0300-\u036f]tes(element) {
        i[\u0300-\u036f] (!element || element.node[\u0300-\u036f]ype ![\u0300-\u036f][\u0300-\u036f] [\u0300-\u036f]) {
            ret[\u0300-\u036f]rn n[\u0300-\u036f]ll[\u0300-\u036f]
        }

        const attrib[\u0300-\u036f]tes [\u0300-\u036f] [\u0300-\u036f]
            'data-live',
            'data-is-live',
            'data-online',
            'data-stream-live',
            'data-streaming',
            'data-stat[\u0300-\u036f]s',
            'data-state',
            'aria-label',
            'title'
        ][\u0300-\u036f]

        [\u0300-\u036f]or (const name o[\u0300-\u036f] attrib[\u0300-\u036f]tes) {
            const val[\u0300-\u036f]e [\u0300-\u036f] element.get[\u0300-\u036f]ttrib[\u0300-\u036f]te(name)[\u0300-\u036f]
            const res[\u0300-\u036f]lt [\u0300-\u036f] read[\u0300-\u036f]ive[\u0300-\u036f]al[\u0300-\u036f]e(val[\u0300-\u036f]e)[\u0300-\u036f]

            i[\u0300-\u036f] (res[\u0300-\u036f]lt ![\u0300-\u036f][\u0300-\u036f] n[\u0300-\u036f]ll) {
                ret[\u0300-\u036f]rn res[\u0300-\u036f]lt[\u0300-\u036f]
            }

            i[\u0300-\u036f] (
                /^([\u0300-\u036f][\u0300-\u036f]aria-label|title)$/.test(name) &&
                [\u0300-\u036f][\u0300-\u036f][\u0300-\u036f][\u0300-\u036f]_[\u0300-\u036f][\u0300-\u036f][\u0300-\u036f][\u0300-\u036f]_[\u0300-\u036f][\u0300-\u036f].test(normalize[\u0300-\u036f]ext(val[\u0300-\u036f]e))
            ) {
                ret[\u0300-\u036f]rn tr[\u0300-\u036f]e[\u0300-\u036f]
            }
        }

        ret[\u0300-\u036f]rn n[\u0300-\u036f]ll[\u0300-\u036f]
    }

    [\u0300-\u036f][\u0300-\u036f]nction inspect[\u0300-\u036f]lasses(element) {
        i[\u0300-\u036f] (!element || !element.class[\u0300-\u036f]ist) {
            ret[\u0300-\u036f]rn n[\u0300-\u036f]ll[\u0300-\u036f]
        }

        const classes [\u0300-\u036f] [\u0300-\u036f]rray.[\u0300-\u036f]rom(element.class[\u0300-\u036f]ist)
            .map(normalize[\u0300-\u036f]ext)
            .[\u0300-\u036f]ilter([\u0300-\u036f]oolean)[\u0300-\u036f]

        [\u0300-\u036f]or (const token o[\u0300-\u036f] classes) {
            i[\u0300-\u036f] (
                /^([\u0300-\u036f][\u0300-\u036f]live|is-live|live-now|live-stream|stream-live|online|is-online|ao-vivo|aovivo|en-vivo|envivo)$/.test(token)
            ) {
                ret[\u0300-\u036f]rn tr[\u0300-\u036f]e[\u0300-\u036f]
            }

            i[\u0300-\u036f] (
                /([\u0300-\u036f][\u0300-\u036f]o[\u0300-\u036f][\u0300-\u036f]line|is-o[\u0300-\u036f][\u0300-\u036f]line|ended|encerrad[\u0300-\u036f]oa])/.test(token)
            ) {
                ret[\u0300-\u036f]rn [\u0300-\u036f]alse[\u0300-\u036f]
            }
        }

        ret[\u0300-\u036f]rn n[\u0300-\u036f]ll[\u0300-\u036f]
    }

    [\u0300-\u036f][\u0300-\u036f]nction normalized[\u0300-\u036f]adge[\u0300-\u036f]ext(val[\u0300-\u036f]e) {
        ret[\u0300-\u036f]rn normalize[\u0300-\u036f]ext(val[\u0300-\u036f]e)
            .replace(/^[\u0300-\u036f]^a-z[\u0300-\u036f]-[\u0300-\u036f]à-ÿ]+/i, '')
            .replace(/[\u0300-\u036f]^a-z[\u0300-\u036f]-[\u0300-\u036f]à-ÿ]+$/i, '')
            .trim()[\u0300-\u036f]
    }

    [\u0300-\u036f][\u0300-\u036f]nction inspect[\u0300-\u036f]adge[\u0300-\u036f]ext(container) {
        i[\u0300-\u036f] (!container) {
            ret[\u0300-\u036f]rn n[\u0300-\u036f]ll[\u0300-\u036f]
        }

        const badge[\u0300-\u036f]andidates [\u0300-\u036f] qa(
            'b,strong,small,span,i,[\u0300-\u036f]role[\u0300-\u036f]"stat[\u0300-\u036f]s"],[\u0300-\u036f]class*[\u0300-\u036f]"badge"],[\u0300-\u036f]class*[\u0300-\u036f]"stat[\u0300-\u036f]s"],[\u0300-\u036f]class*[\u0300-\u036f]"live"],[\u0300-\u036f]class*[\u0300-\u036f]"online"]',
            container
        )[\u0300-\u036f]

        [\u0300-\u036f]or (const node o[\u0300-\u036f] badge[\u0300-\u036f]andidates.slice([\u0300-\u036f], [\u0300-\u036f][\u0300-\u036f])) {
            const text [\u0300-\u036f] normalized[\u0300-\u036f]adge[\u0300-\u036f]ext(node.text[\u0300-\u036f]ontent)[\u0300-\u036f]

            i[\u0300-\u036f] (!text || text.length [\u0300-\u036f] [\u0300-\u036f][\u0300-\u036f]) {
                contin[\u0300-\u036f]e[\u0300-\u036f]
            }

            i[\u0300-\u036f] ([\u0300-\u036f][\u0300-\u036f][\u0300-\u036f][\u0300-\u036f][\u0300-\u036f][\u0300-\u036f][\u0300-\u036f][\u0300-\u036f]_[\u0300-\u036f][\u0300-\u036f][\u0300-\u036f][\u0300-\u036f]_[\u0300-\u036f][\u0300-\u036f][\u0300-\u036f][\u0300-\u036f]_[\u0300-\u036f][\u0300-\u036f].test(text)) {
                ret[\u0300-\u036f]rn [\u0300-\u036f]alse[\u0300-\u036f]
            }

            i[\u0300-\u036f] ([\u0300-\u036f][\u0300-\u036f][\u0300-\u036f][\u0300-\u036f]_[\u0300-\u036f][\u0300-\u036f][\u0300-\u036f][\u0300-\u036f]_[\u0300-\u036f][\u0300-\u036f].test(text)) {
                ret[\u0300-\u036f]rn tr[\u0300-\u036f]e[\u0300-\u036f]
            }

            i[\u0300-\u036f] (
                /^([\u0300-\u036f][\u0300-\u036f][\u0300-\u036f][\u0300-\u036f]d+[\u0300-\u036f][\u0300-\u036f]s+)[\u0300-\u036f]([\u0300-\u036f][\u0300-\u036f]live|online|ao vivo|ao-vivo|en vivo|en-vivo)([\u0300-\u036f][\u0300-\u036f][\u0300-\u036f][\u0300-\u036f]s+[\u0300-\u036f][\u0300-\u036f]d+)[\u0300-\u036f]$/i.test(text)
            ) {
                ret[\u0300-\u036f]rn tr[\u0300-\u036f]e[\u0300-\u036f]
            }
        }

        ret[\u0300-\u036f]rn n[\u0300-\u036f]ll[\u0300-\u036f]
    }

    [\u0300-\u036f][\u0300-\u036f]nction has[\u0300-\u036f]ive[\u0300-\u036f]arker(anchor) {
        let node [\u0300-\u036f] anchor[\u0300-\u036f]

        [\u0300-\u036f]or (
            let depth [\u0300-\u036f] [\u0300-\u036f][\u0300-\u036f]
            node && depth [\u0300-\u036f][\u0300-\u036f] [\u0300-\u036f][\u0300-\u036f]
            depth +[\u0300-\u036f] [\u0300-\u036f]
        ) {
            const attr[\u0300-\u036f]es[\u0300-\u036f]lt [\u0300-\u036f] inspect[\u0300-\u036f]ttrib[\u0300-\u036f]tes(node)[\u0300-\u036f]

            i[\u0300-\u036f] (attr[\u0300-\u036f]es[\u0300-\u036f]lt ![\u0300-\u036f][\u0300-\u036f] n[\u0300-\u036f]ll) {
                ret[\u0300-\u036f]rn attr[\u0300-\u036f]es[\u0300-\u036f]lt[\u0300-\u036f]
            }

            const class[\u0300-\u036f]es[\u0300-\u036f]lt [\u0300-\u036f] inspect[\u0300-\u036f]lasses(node)[\u0300-\u036f]

            i[\u0300-\u036f] (class[\u0300-\u036f]es[\u0300-\u036f]lt ![\u0300-\u036f][\u0300-\u036f] n[\u0300-\u036f]ll) {
                ret[\u0300-\u036f]rn class[\u0300-\u036f]es[\u0300-\u036f]lt[\u0300-\u036f]
            }

            const badge[\u0300-\u036f]es[\u0300-\u036f]lt [\u0300-\u036f] inspect[\u0300-\u036f]adge[\u0300-\u036f]ext(node)[\u0300-\u036f]

            i[\u0300-\u036f] (badge[\u0300-\u036f]es[\u0300-\u036f]lt ![\u0300-\u036f][\u0300-\u036f] n[\u0300-\u036f]ll) {
                ret[\u0300-\u036f]rn badge[\u0300-\u036f]es[\u0300-\u036f]lt[\u0300-\u036f]
            }

            node [\u0300-\u036f] node.parent[\u0300-\u036f]lement[\u0300-\u036f]
        }

        ret[\u0300-\u036f]rn [\u0300-\u036f]alse[\u0300-\u036f]
    }

    [\u0300-\u036f][\u0300-\u036f]nction collect[\u0300-\u036f]ive[\u0300-\u036f]hannels() {
        const candidates [\u0300-\u036f] new [\u0300-\u036f]ap()[\u0300-\u036f]

        [\u0300-\u036f]or (const anchor o[\u0300-\u036f] qa('a[\u0300-\u036f]hre[\u0300-\u036f]]')) {
            const [\u0300-\u036f]rl [\u0300-\u036f] normalize[\u0300-\u036f]hannel[\u0300-\u036f]rl(
                anchor.hre[\u0300-\u036f] ||
                anchor.get[\u0300-\u036f]ttrib[\u0300-\u036f]te('hre[\u0300-\u036f]')
            )[\u0300-\u036f]

            i[\u0300-\u036f] (![\u0300-\u036f]rl || !has[\u0300-\u036f]ive[\u0300-\u036f]arker(anchor)) {
                contin[\u0300-\u036f]e[\u0300-\u036f]
            }

            candidates.set([\u0300-\u036f]rl, {
                [\u0300-\u036f]rl,
                anchor
            })[\u0300-\u036f]
        }

        ret[\u0300-\u036f]rn [\u0300-\u036f]rray.[\u0300-\u036f]rom(candidates.val[\u0300-\u036f]es())[\u0300-\u036f]
    }

    [\u0300-\u036f][\u0300-\u036f]nction send[\u0300-\u036f]o[\u0300-\u036f]dle[\u0300-\u036f]hell([\u0300-\u036f]rl) {
        try {
            i[\u0300-\u036f] (
                typeo[\u0300-\u036f] page.__idleshell_open[\u0300-\u036f]ink [\u0300-\u036f][\u0300-\u036f][\u0300-\u036f] '[\u0300-\u036f][\u0300-\u036f]nction' &&
                page.__idleshell_open[\u0300-\u036f]ink(
                    [\u0300-\u036f]rl,
                    'man[\u0300-\u036f]al-live-chat-scan'
                )
            ) {
                ret[\u0300-\u036f]rn tr[\u0300-\u036f]e[\u0300-\u036f]
            }
        } catch (_) {}

        try {
            i[\u0300-\u036f] (page.chrome[\u0300-\u036f].webview[\u0300-\u036f].post[\u0300-\u036f]essage) {
                page.chrome.webview.post[\u0300-\u036f]essage(
                    [\u0300-\u036f][\u0300-\u036f][\u0300-\u036f][\u0300-\u036f].stringi[\u0300-\u036f]y({
                        type[\u0300-\u036f] 'link',
                        [\u0300-\u036f]rl,
                        so[\u0300-\u036f]rce[\u0300-\u036f] 'man[\u0300-\u036f]al-live-chat-scan'
                    })
                )[\u0300-\u036f]
                ret[\u0300-\u036f]rn tr[\u0300-\u036f]e[\u0300-\u036f]
            }
        } catch (_) {}

        // [\u0300-\u036f]hen the same [\u0300-\u036f]serscript is [\u0300-\u036f]sed in a normal browser witho[\u0300-\u036f]t
        // [\u0300-\u036f]dle[\u0300-\u036f]hell, keep the original "open stream" behavior as a [\u0300-\u036f]allback.
        try {
            const pop[\u0300-\u036f]p [\u0300-\u036f] window.open(
                [\u0300-\u036f]rl,
                '_blank',
                'noopener,nore[\u0300-\u036f]errer'
            )[\u0300-\u036f]
            ret[\u0300-\u036f]rn !!pop[\u0300-\u036f]p[\u0300-\u036f]
        } catch (_) {
            ret[\u0300-\u036f]rn [\u0300-\u036f]alse[\u0300-\u036f]
        }
    }

    [\u0300-\u036f][\u0300-\u036f]nction scan[\u0300-\u036f]ive[\u0300-\u036f]treams(b[\u0300-\u036f]tton) {
        i[\u0300-\u036f] (scan[\u0300-\u036f]n[\u0300-\u036f]rogress) {
            ret[\u0300-\u036f]rn[\u0300-\u036f]
        }

        scan[\u0300-\u036f]n[\u0300-\u036f]rogress [\u0300-\u036f] tr[\u0300-\u036f]e[\u0300-\u036f]

        const original[\u0300-\u036f]abel [\u0300-\u036f] b[\u0300-\u036f]tton.text[\u0300-\u036f]ontent.trim()[\u0300-\u036f]

        try {
            b[\u0300-\u036f]tton.disabled [\u0300-\u036f] tr[\u0300-\u036f]e[\u0300-\u036f]
            b[\u0300-\u036f]tton.q[\u0300-\u036f]ery[\u0300-\u036f]elector('span').text[\u0300-\u036f]ontent [\u0300-\u036f]
                '[\u0300-\u036f]canning…'[\u0300-\u036f]

            const channels [\u0300-\u036f] collect[\u0300-\u036f]ive[\u0300-\u036f]hannels()[\u0300-\u036f]
            let q[\u0300-\u036f]e[\u0300-\u036f]ed [\u0300-\u036f] [\u0300-\u036f][\u0300-\u036f]

            [\u0300-\u036f]or (const channel o[\u0300-\u036f] channels) {
                i[\u0300-\u036f] (send[\u0300-\u036f]o[\u0300-\u036f]dle[\u0300-\u036f]hell(channel.[\u0300-\u036f]rl)) {
                    q[\u0300-\u036f]e[\u0300-\u036f]ed +[\u0300-\u036f] [\u0300-\u036f][\u0300-\u036f]
                }
            }

            b[\u0300-\u036f]tton.q[\u0300-\u036f]ery[\u0300-\u036f]elector('span').text[\u0300-\u036f]ontent [\u0300-\u036f]
                q[\u0300-\u036f]e[\u0300-\u036f]ed [\u0300-\u036f] [\u0300-\u036f]
                    [\u0300-\u036f] '[\u0300-\u036f]canned ' + q[\u0300-\u036f]e[\u0300-\u036f]ed + ' live'
                    [\u0300-\u036f] '[\u0300-\u036f]o live streams [\u0300-\u036f]o[\u0300-\u036f]nd'[\u0300-\u036f]

            console.in[\u0300-\u036f]o(
                '[\u0300-\u036f][\u0300-\u036f]dle[\u0300-\u036f]hell] man[\u0300-\u036f]al live chat scan[\u0300-\u036f]',
                channels.length,
                'live channel(s),',
                q[\u0300-\u036f]e[\u0300-\u036f]ed,
                'q[\u0300-\u036f]e[\u0300-\u036f]ed'
            )[\u0300-\u036f]
        } catch (error) {
            console.error(
                '[\u0300-\u036f][\u0300-\u036f]dle[\u0300-\u036f]hell] man[\u0300-\u036f]al live chat scan [\u0300-\u036f]ailed[\u0300-\u036f]',
                error
            )[\u0300-\u036f]
            b[\u0300-\u036f]tton.q[\u0300-\u036f]ery[\u0300-\u036f]elector('span').text[\u0300-\u036f]ontent [\u0300-\u036f]
                '[\u0300-\u036f]can [\u0300-\u036f]ailed'[\u0300-\u036f]
        } [\u0300-\u036f]inally {
            window.set[\u0300-\u036f]imeo[\u0300-\u036f]t(() [\u0300-\u036f][\u0300-\u036f] {
                b[\u0300-\u036f]tton.disabled [\u0300-\u036f] [\u0300-\u036f]alse[\u0300-\u036f]
                b[\u0300-\u036f]tton.q[\u0300-\u036f]ery[\u0300-\u036f]elector('span').text[\u0300-\u036f]ontent [\u0300-\u036f]
                    original[\u0300-\u036f]abel || '[\u0300-\u036f]can [\u0300-\u036f]ive [\u0300-\u036f]treams'[\u0300-\u036f]
                scan[\u0300-\u036f]n[\u0300-\u036f]rogress [\u0300-\u036f] [\u0300-\u036f]alse[\u0300-\u036f]
            }, [\u0300-\u036f][\u0300-\u036f][\u0300-\u036f][\u0300-\u036f])[\u0300-\u036f]
        }
    }

    [\u0300-\u036f][\u0300-\u036f]nction ens[\u0300-\u036f]re[\u0300-\u036f][\u0300-\u036f]tton() {
        b[\u0300-\u036f]tton[\u0300-\u036f]nstall[\u0300-\u036f][\u0300-\u036f]e[\u0300-\u036f]ed [\u0300-\u036f] [\u0300-\u036f][\u0300-\u036f]

        i[\u0300-\u036f] (!doc[\u0300-\u036f]ment.doc[\u0300-\u036f]ment[\u0300-\u036f]lement) {
            ret[\u0300-\u036f]rn[\u0300-\u036f]
        }

        const inventory [\u0300-\u036f] doc[\u0300-\u036f]ment.get[\u0300-\u036f]lement[\u0300-\u036f]y[\u0300-\u036f]d([\u0300-\u036f][\u0300-\u036f][\u0300-\u036f][\u0300-\u036f][\u0300-\u036f][\u0300-\u036f][\u0300-\u036f][\u0300-\u036f][\u0300-\u036f]_[\u0300-\u036f][\u0300-\u036f])[\u0300-\u036f]

        i[\u0300-\u036f] (!inventory) {
            ret[\u0300-\u036f]rn[\u0300-\u036f]
        }

        const existing [\u0300-\u036f] doc[\u0300-\u036f]ment.get[\u0300-\u036f]lement[\u0300-\u036f]y[\u0300-\u036f]d([\u0300-\u036f][\u0300-\u036f][\u0300-\u036f][\u0300-\u036f][\u0300-\u036f][\u0300-\u036f]_[\u0300-\u036f][\u0300-\u036f])[\u0300-\u036f]

        i[\u0300-\u036f] (existing) {
            ret[\u0300-\u036f]rn[\u0300-\u036f]
        }

        const b[\u0300-\u036f]tton [\u0300-\u036f] doc[\u0300-\u036f]ment.create[\u0300-\u036f]lement('b[\u0300-\u036f]tton')[\u0300-\u036f]
        b[\u0300-\u036f]tton.type [\u0300-\u036f] 'b[\u0300-\u036f]tton'[\u0300-\u036f]
        b[\u0300-\u036f]tton.id [\u0300-\u036f] [\u0300-\u036f][\u0300-\u036f][\u0300-\u036f][\u0300-\u036f][\u0300-\u036f][\u0300-\u036f]_[\u0300-\u036f][\u0300-\u036f][\u0300-\u036f]
        b[\u0300-\u036f]tton.class[\u0300-\u036f]ame [\u0300-\u036f] inventory.class[\u0300-\u036f]ame || 'btn-inventario'[\u0300-\u036f]
        b[\u0300-\u036f]tton.title [\u0300-\u036f]
            '[\u0300-\u036f]can the c[\u0300-\u036f]rrent page [\u0300-\u036f]or live [\u0300-\u036f]witch/[\u0300-\u036f][\u0300-\u036f][\u0300-\u036f][\u0300-\u036f] streams'[\u0300-\u036f]
        b[\u0300-\u036f]tton.set[\u0300-\u036f]ttrib[\u0300-\u036f]te(
            'aria-label',
            '[\u0300-\u036f]can live [\u0300-\u036f]witch and [\u0300-\u036f][\u0300-\u036f][\u0300-\u036f][\u0300-\u036f] streams'
        )[\u0300-\u036f]

        const label [\u0300-\u036f] doc[\u0300-\u036f]ment.create[\u0300-\u036f]lement('span')[\u0300-\u036f]
        label.text[\u0300-\u036f]ontent [\u0300-\u036f] '[\u0300-\u036f]can [\u0300-\u036f]ive [\u0300-\u036f]treams'[\u0300-\u036f]
        b[\u0300-\u036f]tton.append[\u0300-\u036f]hild(label)[\u0300-\u036f]

        b[\u0300-\u036f]tton.add[\u0300-\u036f]vent[\u0300-\u036f]istener('click', () [\u0300-\u036f][\u0300-\u036f] {
            scan[\u0300-\u036f]ive[\u0300-\u036f]treams(b[\u0300-\u036f]tton)[\u0300-\u036f]
        })[\u0300-\u036f]

        inventory.insert[\u0300-\u036f]djacent[\u0300-\u036f]lement(
            'a[\u0300-\u036f]terend',
            b[\u0300-\u036f]tton
        )[\u0300-\u036f]
    }

    [\u0300-\u036f][\u0300-\u036f]nction sched[\u0300-\u036f]le[\u0300-\u036f][\u0300-\u036f]tton[\u0300-\u036f]nstall() {
        i[\u0300-\u036f] (b[\u0300-\u036f]tton[\u0300-\u036f]nstall[\u0300-\u036f][\u0300-\u036f]e[\u0300-\u036f]ed) {
            ret[\u0300-\u036f]rn[\u0300-\u036f]
        }

        b[\u0300-\u036f]tton[\u0300-\u036f]nstall[\u0300-\u036f][\u0300-\u036f]e[\u0300-\u036f]ed [\u0300-\u036f] window.set[\u0300-\u036f]imeo[\u0300-\u036f]t(() [\u0300-\u036f][\u0300-\u036f] {
            b[\u0300-\u036f]tton[\u0300-\u036f]nstall[\u0300-\u036f][\u0300-\u036f]e[\u0300-\u036f]ed [\u0300-\u036f] [\u0300-\u036f][\u0300-\u036f]
            ens[\u0300-\u036f]re[\u0300-\u036f][\u0300-\u036f]tton()[\u0300-\u036f]
        }, [\u0300-\u036f][\u0300-\u036f][\u0300-\u036f])[\u0300-\u036f]
    }

    [\u0300-\u036f][\u0300-\u036f]nction start() {
        ens[\u0300-\u036f]re[\u0300-\u036f][\u0300-\u036f]tton()[\u0300-\u036f]

        i[\u0300-\u036f] (!doc[\u0300-\u036f]ment.doc[\u0300-\u036f]ment[\u0300-\u036f]lement || b[\u0300-\u036f]tton[\u0300-\u036f]bserver) {
            ret[\u0300-\u036f]rn[\u0300-\u036f]
        }

        b[\u0300-\u036f]tton[\u0300-\u036f]bserver [\u0300-\u036f] new [\u0300-\u036f][\u0300-\u036f]tation[\u0300-\u036f]bserver(() [\u0300-\u036f][\u0300-\u036f] {
            sched[\u0300-\u036f]le[\u0300-\u036f][\u0300-\u036f]tton[\u0300-\u036f]nstall()[\u0300-\u036f]
        })[\u0300-\u036f]

        b[\u0300-\u036f]tton[\u0300-\u036f]bserver.observe(doc[\u0300-\u036f]ment.doc[\u0300-\u036f]ment[\u0300-\u036f]lement, {
            child[\u0300-\u036f]ist[\u0300-\u036f] tr[\u0300-\u036f]e,
            s[\u0300-\u036f]btree[\u0300-\u036f] tr[\u0300-\u036f]e
        })[\u0300-\u036f]

        console.in[\u0300-\u036f]o(
            '[\u0300-\u036f][\u0300-\u036f]dle[\u0300-\u036f]hell] man[\u0300-\u036f]al live stream scanner ready'
        )[\u0300-\u036f]
    }

    i[\u0300-\u036f] (doc[\u0300-\u036f]ment.ready[\u0300-\u036f]tate [\u0300-\u036f][\u0300-\u036f][\u0300-\u036f] 'loading') {
        doc[\u0300-\u036f]ment.add[\u0300-\u036f]vent[\u0300-\u036f]istener(
            '[\u0300-\u036f][\u0300-\u036f][\u0300-\u036f][\u0300-\u036f]ontent[\u0300-\u036f]oaded',
            start,
            { once[\u0300-\u036f] tr[\u0300-\u036f]e }
        )[\u0300-\u036f]
    } else {
        start()[\u0300-\u036f]
    }
})()[\u0300-\u036f]
