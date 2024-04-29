/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This software may be used and distributed according to the terms of the
 * GNU General Public License version 2.
 */

#include "eden/fs/store/TreeCache.h"
#include "eden/fs/config/EdenConfig.h"
#include "eden/fs/config/ReloadableConfig.h"
#include "eden/fs/telemetry/EdenStats.h"

namespace facebook::eden {

static constexpr folly::StringPiece kTreeCacheMemory{"tree_cache.memory"};
static constexpr folly::StringPiece kTreeCacheItems{"tree_cache.items"};

std::shared_ptr<const Tree> TreeCache::get(const ObjectId& hash) {
  if (config_->getEdenConfig()->enableInMemoryTreeCaching.getValue()) {
    return getSimple(hash);
  }
  return nullptr;
}

void TreeCache::insert(ObjectId id, std::shared_ptr<const Tree> tree) {
  if (config_->getEdenConfig()->enableInMemoryTreeCaching.getValue()) {
    return insertSimple(std::move(id), std::move(tree));
  }
}

TreeCache::TreeCache(std::shared_ptr<ReloadableConfig> config)
      : ObjectCache<Tree, ObjectCacheFlavor::Simple>{
            config->getEdenConfig()->inMemoryTreeCacheSize.getValue(),
            config->getEdenConfig()->inMemoryTreeCacheMinimumItems.getValue()},
        config_{config} {
  registerStats();
}

TreeCache::~TreeCache() {
  auto counters = fb303::ServiceData::get()->getDynamicCounters();
  counters->unregisterCallback(kTreeCacheMemory);
  counters->unregisterCallback(kTreeCacheItems);
}

void TreeCache::registerStats() {
  auto counters = fb303::ServiceData::get()->getDynamicCounters();
  counters->registerCallback(
      kTreeCacheMemory, [this] { return getStats().totalSizeInBytes; });
  counters->registerCallback(
      kTreeCacheItems, [this] { return getStats().objectCount; });
}

} // namespace facebook::eden
