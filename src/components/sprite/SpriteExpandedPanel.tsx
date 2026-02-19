import { useState } from 'react'
import type { SpriteInfo } from '../../types/sprite'
import { InfoTab }        from './tabs/InfoTab'
import { CheckpointTab }  from './tabs/CheckpointTab'
import { SessionTab }     from './tabs/SessionTab'
import { ServiceTab }     from './tabs/ServiceTab'

const TABS = ['Info', 'Checkpoints', 'Sessions', 'Services'] as const
type Tab = typeof TABS[number]

interface Props { sprite: SpriteInfo }

export function SpriteExpandedPanel({ sprite }: Props) {
  const [activeTab, setActiveTab] = useState<Tab>('Info')

  return (
    <div className="border-t border-zinc-800 mt-1">
      {/* Tab bar */}
      <div className="flex border-b border-zinc-800 px-3">
        {TABS.map(tab => (
          <button
            key={tab}
            onClick={() => setActiveTab(tab)}
            className={`px-3 py-2 text-xs font-medium transition-colors border-b-2 -mb-px ${
              activeTab === tab
                ? 'border-zinc-400 text-zinc-100'
                : 'border-transparent text-zinc-500 hover:text-zinc-300'
            }`}
          >
            {tab}
          </button>
        ))}
      </div>

      {/* Tab content */}
      <div className="p-3">
        {activeTab === 'Info'        && <InfoTab       sprite={sprite} />}
        {activeTab === 'Checkpoints' && <CheckpointTab sprite={sprite} />}
        {activeTab === 'Sessions'    && <SessionTab    sprite={sprite} />}
        {activeTab === 'Services'    && <ServiceTab    sprite={sprite} />}
      </div>
    </div>
  )
}
